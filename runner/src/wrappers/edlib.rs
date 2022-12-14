use super::*;

use edlib_rs::edlibrs::*;

pub struct Edlib;

impl AlignerParams for EdlibParams {
    type Aligner = Edlib;

    fn new(self, cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner {
        Self::default(cm, trace, max_len)
    }

    fn default(cm: CostModel, _trace: bool, _max_len: usize) -> Self::Aligner {
        assert!(cm.is_unit());
        Edlib
    }
}

impl Aligner for Edlib {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        (
            -edlibAlignRs(a, b, &EdlibAlignConfigRs::default()).editDistance,
            None,
        )
    }
}
