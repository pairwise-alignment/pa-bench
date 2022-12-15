use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};

use pa_generate::*;
use pa_types::*;

mod algorithms;
pub use crate::algorithms::*;

/// Metadata for a generated file. When a method fails on a dataset, all
/// datasets with the same `error_model` and larger `error_rate` and/or `length`
/// are skipped.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct DatasetMetadata {
    pub error_model: ErrorModel,
    pub error_rate: f32,
    pub length: usize,
}

/// We promise that the `f32` error rate will never be NaN.
impl Eq for DatasetMetadata {}

/// An alignment job: a single task for the runner to execute and benchmark.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
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
    /// This is used to skip larger jobs after a smaller one fails.
    pub meta: Option<DatasetMetadata>,
}

impl Job {
    /// Whether this job is larger than another job.
    pub fn is_larger(&self, o: &Self) -> bool {
        let self_meta = self.meta.as_ref().unwrap();
        let other_meta = o.meta.as_ref().unwrap();
        self.costs == o.costs
            && self.traceback == o.traceback
            && self.algo == o.algo
            && self_meta.error_model == other_meta.error_model
            && self_meta.error_rate >= other_meta.error_rate
            && self_meta.length >= other_meta.length
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Measured {
    pub runtime: Duration,
    pub memory: Bytes,
    pub cpu_freq_start: Option<f32>,
    pub cpu_freq_end: Option<f32>,
    pub cpu_clocks: Option<u64>,
}

/// The output of an alignment job.
#[derive(Serialize, Deserialize, Debug)]
pub struct JobOutput {
    pub costs: Vec<Cost>,
    //pub cigars: Vec<Cigar>,
    pub measured: Measured,
}

/// The result of an alignment job, containing the input and output.
#[derive(Serialize, Deserialize, Debug)]
pub struct JobResult {
    pub job: Job,
    // TODO(ragnar): Make this a result with a specific error type that indicates the failure reason.
    pub output: Option<JobOutput>,
}
