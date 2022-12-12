use super::*;

use parasailors::*;

pub struct ParasailStripedWrapper {
    matrix: Matrix,
    gap_open: i32,
    gap_extend: i32,
}

impl ParasailStripedWrapper {
    pub fn new(
        _max_len: usize,
        _params: ParasailStripedParams,
        costs: CostModel,
        _traceback: bool,
    ) -> Self {
        let (r#match, sub, open, extend) = match costs {
            CostModel::Linear {
                r#match,
                sub,
                indel,
            } => (r#match, sub, 0, indel),
            CostModel::Affine {
                r#match,
                sub,
                open,
                extend,
            } => (r#match, sub, open, extend),
            _ => unimplemented!(),
        };
        Self {
            matrix: Matrix::create("ACGT", r#match as _, -sub as _),
            gap_open: open + extend,
            gap_extend: extend,
        }
    }
}

impl Wrapper for ParasailStripedWrapper {
    fn cost(&mut self, a: Seq, b: Seq) -> Cost {
        let a = Profile::new(a, &self.matrix);
        global_alignment_score(&a, b, self.gap_open, self.gap_extend)
    }

    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Cigar) {
        todo!()
    }
}
