use pa_bench_types::*;
use pa_types::*;

mod block_aligner;
mod edlib;
mod ksw2;
mod parasail;
mod triple_accel;
mod wfa;

/// Parameters for an aligner, with a `new` method to instantiate the aligner.
pub trait AlignerParams {
    type Aligner: Aligner;

    /// Instantiate the aligner with a configuration.
    fn new(&self, cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner;

    /// Is the aligner exact?
    fn is_exact(&self) -> bool;
}

/// Generic pairwise global alignment interface.
pub trait Aligner {
    /// An alignment of sequences `a` and `b`.
    /// The returned cost is the *non-negative* cost of the alignment.
    /// Returns a trace when specified on construction.
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>);
}

/// Get an instance of the corresponding wrapper based on the algorithm.
pub fn get_aligner(
    algo: AlgorithmParams,
    cm: CostModel,
    trace: bool,
    max_len: usize,
) -> (Box<dyn Aligner>, bool) {
    use AlgorithmParams::*;
    match algo {
        BlockAligner(params) => (Box::new(params.new(cm, trace, max_len)), params.is_exact()),
        ParasailStriped(params) => (Box::new(params.new(cm, trace, max_len)), params.is_exact()),
        Edlib(params) => (Box::new(params.new(cm, trace, max_len)), params.is_exact()),
        TripleAccel(params) => (Box::new(params.new(cm, trace, max_len)), params.is_exact()),
        Wfa(params) => (Box::new(params.new(cm, trace, max_len)), params.is_exact()),
        Ksw2(params) => (Box::new(params.new(cm, trace, max_len)), params.is_exact()),
    }
}
