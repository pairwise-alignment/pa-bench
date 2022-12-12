use serde::{Deserialize, Serialize};

/// Which algorithm to run and benchmark, along with algorithm-specific parameters.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AlgorithmParams {
    BlockAligner(BlockAlignerParams),
    ParasailStriped(ParasailStripedParams),
    // Add more algorithms here!
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockAlignerParams {
    pub min_size: usize,
    pub max_size: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParasailStripedParams;
