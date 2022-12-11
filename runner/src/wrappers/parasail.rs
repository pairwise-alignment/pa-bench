use super::*;

pub struct ParasailStripedWrapper {}

impl ParasailStripedWrapper {
    pub fn new(
        max_len: usize,
        params: ParasailStripedParams,
        costs: CostModel,
        traceback: bool,
    ) -> Self {
        todo!()
    }
}

impl Wrapper for ParasailStripedWrapper {
    fn cost(&mut self, a: Seq, b: Seq) -> Cost {
        todo!()
    }

    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Cigar) {
        todo!()
    }
}
