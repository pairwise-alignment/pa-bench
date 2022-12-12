use pa_bench_types::*;
use pa_types::*;

mod block_aligner;
mod parasail;

pub trait AlignerParams {
    type Aligner: Aligner;

    /// Instantiate the aligner with a configuration.
    fn new(self, cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner;

    /// Instantiate the aligner with a default configuration.
    fn default(_cm: CostModel, _trace: bool, _max_len: usize) -> Self::Aligner {
        unimplemented!("This aligner does not support default parameters.");
    }
}

/// Generic pairwise global alignment interface.
pub trait Aligner {
    /// An alignment of sequences `a` and `b`.
    /// Returns a trace when specified on construction.
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>);
}

/// Get an instance of the corresponding wrapper based on the algorithm.
pub fn get_aligner(
    algo: AlgorithmParams,
    cm: CostModel,
    trace: bool,
    max_len: usize,
) -> Box<dyn Aligner> {
    use AlgorithmParams::*;
    match algo {
        BlockAligner(params) => Box::new(params.new(cm, trace, max_len)),
        ParasailStriped(params) => Box::new(params.new(cm, trace, max_len)),
    }
}
