use super::*;

#[rustfmt::skip]
use ::triple_accel::*;

pub struct TripleAccel {
    costs: ::triple_accel::levenshtein::EditCosts,
    trace: bool,
}

impl AlignerParams for TripleAccelParams {
    type Aligner = TripleAccel;

    fn new(&self, cm: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        // TripleAccel has a bug in traceback when using affine gap costs.
        assert!(!cm.is_affine());
        let costs = ::triple_accel::levenshtein::EditCosts::new(
            cm.sub as _,
            cm.extend as _,
            cm.open as _,
            None,
        );
        Self::Aligner { costs, trace }
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl Aligner for TripleAccel {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let (cost, edits) =
            ::triple_accel::levenshtein::levenshtein_exp_with_opts(a, b, self.trace, self.costs);

        let cigar = edits.map(|edits| Cigar {
            ops: edits
                .into_iter()
                .map(|edit| CigarElem {
                    op: match edit.edit {
                        EditType::Match => CigarOp::Match,
                        EditType::Mismatch => CigarOp::Sub,
                        EditType::AGap => CigarOp::Ins,
                        EditType::BGap => CigarOp::Del,
                        EditType::Transpose => unimplemented!(),
                    },
                    cnt: edit.count as _,
                })
                .collect(),
        });

        (cost as _, cigar)
    }
}
