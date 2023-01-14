use astar_pairwise_aligner::{visualizer::Visualizer, *};

use super::*;

impl<V: Visualizer + 'static> AlignerParams for AstarPaParams<V> {
    type Aligner = Box<dyn AstarPaAligner>;

    fn new(&self, cm: CostModel, _trace: bool, _max_len: usize) -> Self::Aligner {
        // The trace parameter is ignored since A*PA gives a trace 'for free' currently.
        assert!(cm.is_unit());
        self.aligner()
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
