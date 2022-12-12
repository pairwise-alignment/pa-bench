use pa_bench_types::*;
use pa_types::*;

mod block_aligner_wrapper;
use block_aligner_wrapper::*;
mod parasail_wrapper;
use parasail_wrapper::*;

/// Generic pairwise global alignment interface.
pub trait Wrapper {
    fn cost(&mut self, a: Seq, b: Seq) -> Cost;

    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Cigar);
}

/// Get an instance of the corresponding wrapper based on the algorithm.
pub fn get_wrapper(
    algo: Algorithm,
    max_len: usize,
    costs: CostModel,
    traceback: bool,
) -> Box<dyn Wrapper> {
    use Algorithm::*;

    match algo {
        BlockAligner(p) => Box::new(BlockAlignerWrapper::new(max_len, p, costs, traceback)),
        ParasailStriped(p) => Box::new(ParasailStripedWrapper::new(max_len, p, costs, traceback)),
    }
}
