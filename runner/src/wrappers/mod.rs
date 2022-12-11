use pa_bench_types::*;
use pa_types::*;

mod block_aligner;
use block_aligner::*;
mod parasail;
use parasail::*;

pub trait Wrapper {
    fn cost(&mut self, a: Seq, b: Seq) -> Cost;

    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Cigar);
}

pub fn get_wrapper(
    algo: Algorithm,
    max_len: usize,
    costs: CostModel,
    traceback: bool,
) -> Box<dyn Wrapper> {
    use pa_bench_types::Algorithm::*;

    match algo {
        BlockAligner(p) => Box::new(BlockAlignerWrapper::new(max_len, p, costs, traceback)),
        ParasailStriped(p) => Box::new(ParasailStripedWrapper::new(max_len, p, costs, traceback)),
    }
}
