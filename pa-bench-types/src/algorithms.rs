use serde::{Deserialize, Serialize};

/// Which algorithm to run and benchmark, along with algorithm-specific parameters.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum AlgorithmParams {
    BlockAligner(BlockAlignerParams),
    ParasailStriped(ParasailStripedParams),
    Edlib(EdlibParams),
    TripleAccel(TripleAccelParams),
    Wfa(WfaParams),
    Ksw2(Ksw2Params),
    AstarPA(astar_pairwise_aligner::AstarPaParamsNoVis),
    // Add more algorithms here!
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockAlignerParams {
    pub min_size: usize,
    pub max_size: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ParasailStripedParams;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EdlibParams;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TripleAccelParams;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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

impl Default for WfaParams {
    fn default() -> Self {
        Self {
            memory_model: rust_wfa2::aligner::MemoryModel::MemoryUltraLow,
            heuristic: rust_wfa2::aligner::Heuristic::None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ksw2Method {
    GlobalGreen,
    GlobalSuzuki,
    #[default]
    GlobalSuzukiSse,
    ExtensionGreen,
    ExtensionSuzukiSse,
    DualAffineExtensionGreen,
    DualAffineExtensionSuzukiSse,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Ksw2Params {
    #[serde(default)]
    pub method: Ksw2Method,
    #[serde(default = "band_doubling_enabled")]
    pub band_doubling: bool,
}
fn band_doubling_enabled() -> bool {
    true
}

impl Default for Ksw2Params {
    fn default() -> Self {
        Ksw2Params {
            method: Ksw2Method::GlobalSuzukiSse,
            band_doubling: true,
        }
    }
}
