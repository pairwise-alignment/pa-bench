use pa_base_algos::nw::AstarNwParams;

use crate::*;

impl AlignerParams for AstarNwParams {
    type Aligner = Box<dyn pa_types::Aligner>;

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

impl Aligner for Box<dyn pa_types::Aligner> {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let (cost, cigar) = pa_types::Aligner::align(self.as_mut(), a, b);
        (cost, cigar, AlignerStats::default())
    }
}
