use super::*;

use ksw2_sys::*;

const M: usize = 4;
pub struct Ksw2 {
    params: Ksw2Params,
    trace: bool,
    encoding: [u8; 256],
    score_model: ScoreModel,
    score_matrix: [i8; (M + 1) * (M + 1)],
    open: i8,
    extend: i8,
}

impl AlignerParams for Ksw2Params {
    type Aligner = Ksw2;

    fn new(self, cost_model: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        let score_model = ScoreModel::from_costs(cost_model);
        let mut score_matrix = [0; (M + 1) * (M + 1)];
        for i in 0..M {
            for j in 0..M {
                score_matrix[((M + 1) * i + j) as usize] =
                    //if i == j { 0 } else { -cost_model.sub as i8 };
                    if i == j { score_model.r#match as i8} else { score_model.sub as i8 };
            }
        }

        let mut encoding: [u8; 256] = [0; 256];
        encoding[b'A' as usize] = 0;
        encoding[b'C' as usize] = 1;
        encoding[b'G' as usize] = 2;
        encoding[b'T' as usize] = 3;
        Self::Aligner {
            params: self,
            trace,
            encoding,
            score_model,
            score_matrix,
            // open: (cost_model.open + cost_model.extend) as _,
            // extend: cost_model.extend as _,
            open: -(score_model.open + score_model.extend) as _,
            extend: -score_model.extend as _,
        }
    }

    fn default(cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner {
        Ksw2Params {
            // TODO: Or GlobalSuzukiSse?
            method: Ksw2Method::ExtensionSuzukiSse,
        }
        .new(cm, trace, max_len)
    }
}

impl Aligner for Ksw2 {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let a_mapped: Vec<u8> = a.iter().map(|x| self.encoding[*x as usize]).collect();
        let b_mapped: Vec<u8> = b.iter().map(|x| self.encoding[*x as usize]).collect();
        unsafe {
            let mut output: ksw_extz_t = std::mem::zeroed();
            // Documentation is at
            // https://github.com/lh3/ksw2/blob/06b2183b0f6646d82f2e3f5884008a1b4582f5b5/ksw2.h#L44.
            // TODO(ragnar): Support the other methods.
            assert_eq!(self.params.method, Ksw2Method::ExtensionGreen);
            ksw_extz(
                // don't use a kmalloc memory pool.
                std::ptr::null_mut(),
                // Input sequences
                a_mapped.len() as i32,
                a_mapped.as_ptr(),
                b_mapped.len() as i32,
                b_mapped.as_ptr(),
                (M + 1) as i8,
                // Scoring matrix and gap penalties
                self.score_matrix.as_ptr(),
                self.open,
                self.extend,
                // band: disabled
                -1,
                // zdrop: disabled
                -1,
                // TODO(ragnar): Figure out what this means.
                // end_bonus, needed for ksw_extz2_sse
                // 0
                // flag:
                // https://github.com/lh3/ksw2/blob/master/ksw2.h#L8
                // (seems like it can just be 0 for our use case)
                if self.trace { 0 } else { 1 },
                &mut output,
            );
            let cigar = self.trace.then(|| {
                // TODO: free output.cigar using the kfree function somehow
                let cigar = std::slice::from_raw_parts_mut(output.cigar, output.n_cigar as usize);
                eprintln!("REsolve cigar..\n");
                Cigar::resolve_matches(
                    cigar.into_iter().rev().map(|&mut val| {
                        let val = (
                            match val & 15 {
                                // NOTE: This Match will be resolved to Match or Sub as needed.
                                0 => CigarOp::Match,
                                1 => CigarOp::Del,
                                2 => CigarOp::Ins,
                                7 => CigarOp::Match,
                                8 => CigarOp::Sub,
                                _ => panic!(),
                            },
                            val / 16,
                        );
                        eprintln!("{val:?}");
                        val
                    }),
                    &a_mapped,
                    &b_mapped,
                )
            });
            let cost = self.score_model.global_cost(output.score, a.len(), b.len());
            eprintln!(
                "{:?}\nscore: {} cost: {cost}",
                self.score_model, output.score
            );
            (cost, cigar)
        }
    }
}
