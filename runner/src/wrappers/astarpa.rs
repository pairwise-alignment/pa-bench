#[rustfmt::skip]
use ::astarpa::*;

use super::*;

impl AlignerParams for AstarPaParams {
    type Aligner = Box<dyn AstarStatsAligner>;

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
        Ok(make_aligner(self.diagonal_transition, &self.heuristic))
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl Aligner for Box<dyn AstarStatsAligner> {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let ((cost, cigar), stats) = AstarStatsAligner::align(self.as_mut(), a, b);
        (
            cost,
            Some(cigar),
            AlignerStats {
                expanded: Some(stats.extended + stats.expanded),
            },
        )
    }
}
