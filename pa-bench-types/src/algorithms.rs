use serde::{Deserialize, Serialize};

/// Which algorithm to run and benchmark, along with algorithm-specific parameters.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgorithmParams {
    BlockAligner(BlockAlignerParams),
    ParasailStriped(ParasailStripedParams),
    Edlib(EdlibParams),
    TripleAccel(TripleAccelParams),
    Wfa(WfaParams),
    Ksw2(Ksw2Params),
    // Add more algorithms here!
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockAlignerParams {
    pub min_size: usize,
    pub max_size: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParasailStripedParams;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdlibParams;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TripleAccelParams;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WfaParams {
    #[serde(default = "WfaParams::default_memory_model")]
    pub memory_model: rust_wfa2::aligner::MemoryModel,
    #[serde(default = "WfaParams::default_heuristic")]
    pub heuristic: rust_wfa2::aligner::Heuristic,
}

impl WfaParams {
    const fn default_memory_model() -> rust_wfa2::aligner::MemoryModel {
        rust_wfa2::aligner::MemoryModel::MemoryUltraLow
    }
    const fn default_heuristic() -> rust_wfa2::aligner::Heuristic {
        rust_wfa2::aligner::Heuristic::None
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ksw2Method {
    GlobalGreen,
    GlobalSuzuki,
    GlobalSuzukiSse,
    ExtensionGreen,
    ExtensionSuzukiSse,
    DualAffineExtensionGreen,
    DualAffineExtensionSuzukiSse,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ksw2Params {
    pub method: Ksw2Method,
    pub band_doubling: bool,
}
