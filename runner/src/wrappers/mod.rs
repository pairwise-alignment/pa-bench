use pa_bench_types::*;
use pa_types::*;

mod astarpa;
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

/// A type-erased helper trait that returns a `dyn Aligner`.
pub trait TypeErasedAlignerParams {
    fn new(&self, cm: CostModel, trace: bool, max_len: usize) -> Box<dyn Aligner>;
    fn is_exact(&self) -> bool;
}
impl<A: Aligner + 'static, T: AlignerParams<Aligner = A>> TypeErasedAlignerParams for T {
    fn new(&self, cm: CostModel, trace: bool, max_len: usize) -> Box<dyn Aligner> {
        Box::new(self.new(cm, trace, max_len))
    }
    fn is_exact(&self) -> bool {
        self.is_exact()
    }
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
    algo: &AlgorithmParams,
    cm: CostModel,
    trace: bool,
    max_len: usize,
) -> (Box<dyn Aligner>, bool) {
    use AlgorithmParams::*;
    let params: &dyn TypeErasedAlignerParams = match algo {
        BlockAligner(params) => params,
        ParasailStriped(params) => params,
        Edlib(params) => params,
        TripleAccel(params) => params,
        Wfa(params) => params,
        Ksw2(params) => params,
        AstarPA(_) => todo!(),
    };
    (params.new(cm, trace, max_len), params.is_exact())
}
