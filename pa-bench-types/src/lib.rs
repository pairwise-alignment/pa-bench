use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use pa_generate::*;
use pa_types::*;

mod algorithms;
pub use crate::algorithms::*;

/// Metadata for a generated file. When a method fails on a dataset, all
/// datasets with the same `error_model` and larger `error_rate` and/or `length`
/// are skipped.
pub type DatasetMetadata = (ErrorModel, f32, usize);

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

    /// Metadata of the dataset.
    /// This is used to skip strictly larger jobs after a smaller one fails.
    pub meta: Option<DatasetMetadata>,
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
    // TODO(ragnar): Make this a result with a specific error type that indicates the failure reason.
    pub output: Option<JobOutput>,
}
