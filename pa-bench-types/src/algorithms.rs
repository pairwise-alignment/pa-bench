use serde::{Deserialize, Serialize};

/// Which algorithm to run and benchmark, along with algorithm-specific parameters.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum AlgorithmParams {
    BlockAligner(BlockAlignerParams),
    ParasailStriped(ParasailStripedParams),
    Edlib(EdlibParams),
    TripleAccel(TripleAccelParams),
    Wfa(WfaParams),
    Ksw2(Ksw2Params),
    // Add more algorithms here!
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BlockAlignerParams {
    pub min_size: usize,
    pub max_size: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ParasailStripedParams;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EdlibParams;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TripleAccelParams;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct WfaParams;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Ksw2Params;
