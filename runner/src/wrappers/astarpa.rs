#[rustfmt::skip]
use ::astarpa::*;

use super::*;

impl AlignerParams for AstarPaParamsNoVis {
    type Aligner = Box<dyn AstarPaAligner>;

    fn new(
        &self,
        cm: CostModel,
        trace: bool,
        _max_len: usize,
    ) -> Result<Self::Aligner, &'static str> {
        // The trace parameter must be true since A*PA gives a trace 'for free' currently.
        if !trace {
            return Err("Trace must be true for A*PA");
        }
        if !cm.is_unit() {
            return Err("A*PA only works for unit cost model");
        }
        Ok(self.aligner())
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl Aligner for Box<dyn AstarPaAligner> {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let (cost, cigar) = AstarPaAligner::align(self.as_mut(), a, b).0;
        (cost, Some(cigar))
    }
}
