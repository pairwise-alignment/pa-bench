use crate::*;
use astarpa::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct AstarPaParams {
    pub diagonal_transition: bool,
    pub heuristic: astarpa::HeuristicParams,
}

impl Default for AstarPaParams {
    fn default() -> Self {
        Self {
            diagonal_transition: true,
            heuristic: astarpa::HeuristicParams::default(),
        }
    }
}

impl AlignerParamsTrait for AstarPaParams {
    type Aligner = Box<dyn AstarStatsAligner>;

    fn build(
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

impl AlignerTrait for Box<dyn AstarStatsAligner> {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let ((cost, cigar), stats) = AstarStatsAligner::align(self.as_mut(), a, b);
        let mut s = AlignerStats::default();
        s.insert("expanded".into(), (stats.extended + stats.expanded) as _);
        s.insert("t_precomp".into(), stats.timing.precomp);
        s.insert("t_astar".into(), stats.timing.astar);
        s.insert("t_traceback".into(), stats.timing.traceback);
        s.insert("t_h".into(), stats.h.h_duration);
        s.insert("t_pruning".into(), stats.h.prune_duration);
        s.insert("t_contours".into(), stats.h.contours_duration);
        s.insert("t_reordering".into(), stats.timing.reordering);

        (cost, Some(cigar), s)
    }
}
