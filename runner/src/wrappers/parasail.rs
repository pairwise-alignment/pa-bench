use super::*;

use parasailors::*;

pub struct ParasailStriped {
    matrix: Matrix,
    gap_open: i32,
    gap_extend: i32,
    s: ScoreModel,
}

impl AlignerParams for ParasailStripedParams {
    type Aligner = ParasailStriped;

    fn default(cm: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        assert!(!trace);
        let s = ScoreModel::from_costs(cm);
        Self::Aligner {
            matrix: Matrix::create("ACGT", s.r#match as _, s.sub as _),
            gap_open: -s.open - s.extend,
            gap_extend: -s.extend,
            s,
        }
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl Aligner for ParasailStriped {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let a_len = a.len();
        let a = Profile::new(a, &self.matrix);
        (
            self.s.global_cost(
                global_alignment_score(&a, b, self.gap_open, self.gap_extend),
                a_len,
                b.len(),
            ),
            None,
        )
    }
}
