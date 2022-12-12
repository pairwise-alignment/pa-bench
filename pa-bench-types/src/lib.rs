use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use pa_types::*;

mod algorithms;
pub use crate::algorithms::*;

/// An alignment job: a single task for the runner to execute and benchmark.
#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    /// Path to a `.seq` file.
    pub dataset: PathBuf,
    /// The cost model to use.
    pub costs: CostModel,
    /// Return the full alignment/cigar?
    pub traceback: bool,
    /// The algorithm/parameters to use.
    pub algo: AlgorithmParams,
}

/// The output of an alignment job.
#[derive(Serialize, Deserialize, Debug)]
pub struct JobOutput {
    pub runtime: Duration,
    pub memory: Bytes,
    pub costs: Vec<Cost>,
    pub cigars: Vec<Cigar>,
}

/// The result of an alignment job, containing the input and output.
#[derive(Serialize, Deserialize, Debug)]
pub struct JobResult {
    pub job: Job,
    pub output: Option<JobOutput>,
}
