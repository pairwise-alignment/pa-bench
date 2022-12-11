use super::*;

pub struct BlockAlignerWrapper {}

impl BlockAlignerWrapper {
    pub fn new(
        max_len: usize,
        params: BlockAlignerParams,
        costs: CostModel,
        traceback: bool,
    ) -> Self {
        todo!()
    }
}

impl Wrapper for BlockAlignerWrapper {
    fn cost(&mut self, a: Seq, b: Seq) -> Cost {
        todo!()
    }

    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Cigar) {
        todo!()
    }
}
