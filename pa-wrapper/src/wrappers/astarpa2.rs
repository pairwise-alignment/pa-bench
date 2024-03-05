pub use astarpa2::AstarPa2Params;

use crate::*;

impl AlignerParamsTrait for AstarPa2Params {
    type Aligner = Box<dyn astarpa2::AstarPa2StatsAligner>;

    fn build(
        &self,
        cm: CostModel,
        trace: bool,
        _max_len: usize,
    ) -> Result<Self::Aligner, &'static str> {
        if !cm.is_unit() {
            return Err("A*NW only works for unit cost model");
        }
        Ok(self.make_aligner(trace))
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl AlignerTrait for Box<dyn astarpa2::AstarPa2StatsAligner> {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let (cost, cigar, stats) = astarpa2::AstarPa2StatsAligner::align(self.as_mut(), a, b);
        let mut s = AlignerStats::default();
        // block stats
        s.insert("fmax_tries".into(), stats.f_max_tries as _);
        s.insert("num_blocks".into(), stats.block_stats.num_blocks as _);
        s.insert(
            "num_incremental_blocks".into(),
            stats.block_stats.num_incremental_blocks as _,
        );
        s.insert(
            "computed_lanes".into(),
            stats.block_stats.computed_lanes as _,
        );
        s.insert("unique_lanes".into(), stats.block_stats.unique_lanes as _);
        // trace stats
        s.insert(
            "dt_trace_tries".into(),
            stats.trace_stats.dt_trace_tries as _,
        );
        s.insert(
            "dt_trace_success".into(),
            stats.trace_stats.dt_trace_success as _,
        );
        s.insert(
            "dt_trace_fallback".into(),
            stats.trace_stats.dt_trace_fallback as _,
        );
        s.insert("fill_tries".into(), stats.trace_stats.fill_success as _);
        s.insert("fill_success".into(), stats.trace_stats.fill_success as _);
        s.insert("fill_fallback".into(), stats.trace_stats.fill_success as _);
        // timing stats
        s.insert("t_precomp".into(), stats.t_precomp.as_secs_f64());
        s.insert("t_jrange".into(), stats.t_j_range.as_secs_f64());
        s.insert("t_fixed_jrange".into(), stats.t_fixed_j_range.as_secs_f64());
        s.insert("t_pruning".into(), stats.t_pruning.as_secs_f64());
        s.insert(
            "t_compute".into(),
            stats.block_stats.t_compute.as_secs_f64(),
        );
        s.insert("t_trace_dt".into(), stats.trace_stats.t_dt.as_secs_f64());
        s.insert(
            "t_trace_fill".into(),
            stats.trace_stats.t_fill.as_secs_f64(),
        );

        (cost, cigar, s)
    }
}
