use super::*;

use parasailors::*;

pub struct ParasailStriped {
    matrix: Matrix,
    gap_open: i32,
    gap_extend: i32,
}

impl AlignerParams for ParasailStripedParams {
    type Aligner = ParasailStriped;

    fn new(self, cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner {
        Self::default(cm, trace, max_len)
    }

    fn default(cm: CostModel, _trace: bool, _max_len: usize) -> Self::Aligner {
        let CostModel {
            r#match,
            sub,
            open,
            extend,
        }: CostModel = cm;
        ParasailStriped {
            matrix: Matrix::create("ACGT", r#match as _, -sub as _),
            gap_open: open + extend,
            gap_extend: extend,
        }
    }
}

impl Aligner for ParasailStriped {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let a = Profile::new(a, &self.matrix);
        (
            global_alignment_score(&a, b, self.gap_open, self.gap_extend),
            None,
        )
    }
}
