use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use pa_types::*;

pub mod wrappers {
    #[cfg(feature = "astarpa")]
    pub mod astarnw;
    #[cfg(feature = "astarpa")]
    pub mod astarpa;
    #[cfg(feature = "block_aligner")]
    pub mod block_aligner;
    #[cfg(feature = "edlib")]
    pub mod edlib;
    #[cfg(feature = "ksw2")]
    pub mod ksw2;
    #[cfg(feature = "parasail")]
    pub mod parasail;
    #[cfg(feature = "triple_accel")]
    pub mod triple_accel;
    #[cfg(feature = "wfa")]
    pub mod wfa;
}

/// Parameters for an aligner, with a `new` method to instantiate the aligner.
trait AlignerParamsTrait {
    type Aligner: AlignerTrait;

    /// Instantiate the aligner with a configuration.
    fn build(
        &self,
        cm: CostModel,
        trace: bool,
        max_len: usize,
    ) -> Result<Self::Aligner, &'static str>;

    /// Is the aligner exact?
    fn is_exact(&self) -> bool;
}

/// Alignment statistics. Stats are summed over all sequence pairs in a dataset.
/// Times are in seconds.
pub type AlignerStats = HashMap<String, f64>;

/// Generic pairwise global alignment interface.
pub trait AlignerTrait {
    /// An alignment of sequences `a` and `b`.
    /// The returned cost is the *non-negative* cost of the alignment.
    /// Returns a trace when specified on construction of the aligner.
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats);
}

/// Which algorithm to run and benchmark, along with algorithm-specific parameters.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, strum::EnumDiscriminants)]
#[strum_discriminants(name(Aligner))]
#[strum_discriminants(derive(clap::ValueEnum))]
pub enum AlignerParams {
    #[cfg(feature = "astarpa")]
    AstarNW(pa_base_algos::nw::AstarNwParams),
    #[cfg(feature = "astarpa")]
    AstarPA(wrappers::astarpa::AstarPaParams),
    #[cfg(feature = "block_aligner")]
    BlockAligner(wrappers::block_aligner::BlockAlignerParams),
    #[cfg(feature = "edlib")]
    Edlib(wrappers::edlib::EdlibParams),
    #[cfg(feature = "ksw2")]
    Ksw2(wrappers::ksw2::Ksw2Params),
    #[cfg(feature = "parasail")]
    ParasailStriped(wrappers::parasail::ParasailStripedParams),
    #[cfg(feature = "triple_accel")]
    TripleAccel(wrappers::triple_accel::TripleAccelParams),
    #[cfg(feature = "wfa")]
    Wfa(wrappers::wfa::WfaParams),
    // Add more algorithms here!
}

impl Aligner {
    pub fn default_params(&self) -> AlignerParams {
        use AlignerParams::*;
        match self {
            #[cfg(feature = "astarpa")]
            Aligner::AstarNW => AstarNW(Default::default()),
            #[cfg(feature = "astarpa")]
            Aligner::AstarPA => AstarPA(Default::default()),
            #[cfg(feature = "block_aligner")]
            Aligner::BlockAligner => BlockAligner(Default::default()),
            #[cfg(feature = "edlib")]
            Aligner::Edlib => Edlib(Default::default()),
            #[cfg(feature = "ksw2")]
            Aligner::Ksw2 => Ksw2(Default::default()),
            #[cfg(feature = "parasail")]
            Aligner::ParasailStriped => ParasailStriped(Default::default()),
            #[cfg(feature = "triple_accel")]
            Aligner::TripleAccel => TripleAccel(Default::default()),
            #[cfg(feature = "wfa")]
            Aligner::Wfa => Wfa(Default::default()),
        }
    }
}

impl AlignerParams {
    /// Get an instance of the corresponding wrapper based on the algorithm.
    ///
    /// The bool indicates whether the aligner is exact.
    pub fn build_aligner(
        &self,
        cm: CostModel,
        trace: bool,
        max_len: usize,
    ) -> (Box<dyn AlignerTrait>, bool) {
        use AlignerParams::*;
        let params: &dyn TypeErasedAlignerParams = match self {
            #[cfg(feature = "astarpa")]
            AstarNW(params) => params,
            #[cfg(feature = "astarpa")]
            AstarPA(params) => params,
            #[cfg(feature = "block_aligner")]
            BlockAligner(params) => params,
            #[cfg(feature = "edlib")]
            Edlib(params) => params,
            #[cfg(feature = "ksw2")]
            Ksw2(params) => params,
            #[cfg(feature = "parasail")]
            ParasailStriped(params) => params,
            #[cfg(feature = "triple_accel")]
            TripleAccel(params) => params,
            #[cfg(feature = "wfa")]
            Wfa(params) => params,
        };
        let aligner = match params.build(cm, trace, max_len) {
            Ok(a) => a,
            Err(err) => {
                eprintln!(
                "\n\nBad aligner parameters:\n algo: {self:?}\n cm: sub={} open={} extend={}\n trace: {trace}\n error: {err}",
                cm.sub, cm.open, cm.extend
            );
                std::process::exit(102);
            }
        };

        (aligner, params.is_exact())
    }
}

/// A type-erased wrapper around `AlignerParams` that returns a `dyn Aligner`
/// instead of a specific type.
trait TypeErasedAlignerParams {
    fn build(
        &self,
        cm: CostModel,
        trace: bool,
        max_len: usize,
    ) -> Result<Box<dyn AlignerTrait>, &'static str>;
    fn is_exact(&self) -> bool;
}
impl<A: AlignerTrait + 'static, T: AlignerParamsTrait<Aligner = A>> TypeErasedAlignerParams for T {
    fn build(
        &self,
        cm: CostModel,
        trace: bool,
        max_len: usize,
    ) -> Result<Box<dyn AlignerTrait>, &'static str> {
        Ok(Box::new(self.build(cm, trace, max_len)?))
    }
    fn is_exact(&self) -> bool {
        self.is_exact()
    }
}
