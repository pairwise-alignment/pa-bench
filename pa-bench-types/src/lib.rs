use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use pa_generate::*;
use pa_types::*;

mod algorithms;
pub use crate::algorithms::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct GeneratedDataset {
    pub prefix: PathBuf,
    pub seed: u64,
    pub error_model: ErrorModel,
    pub error_rate: f32,
    pub length: usize,
    pub total_size: usize,
    pub pattern_length: Option<usize>,
}

impl GeneratedDataset {
    pub fn is_larger_than(&self, o: &Self) -> bool {
        self.error_model == o.error_model
            && self.pattern_length == o.pattern_length
            && self.error_rate >= o.error_rate
            && self.length >= o.length
            && self.total_size >= o.total_size
    }

    pub fn path(&self) -> PathBuf {
        self.prefix.join(format!(
            "{:?}-t{}-n{}-e{}.seq",
            self.error_model, self.total_size, self.length, self.error_rate
        ))
    }

    pub fn to_generator(&self) -> DatasetGenerator {
        let Self {
            seed,
            error_model,
            error_rate,
            length,
            total_size,
            pattern_length,
            ..
        } = *self;
        DatasetGenerator {
            settings: SeqPairGenerator {
                length,
                error_rate,
                error_model,
                pattern_length,
            },
            seed: Some(seed),
            cnt: None,
            size: Some(total_size),
        }
    }
}

/// A dataset can be specified in several ways.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Dataset {
    /// Options to deterministically generate a dataset.
    Generated(GeneratedDataset),
    /// Path to a .seq file.
    File(PathBuf),
    /// The data itself.
    /// NOTE: Only use this for testing small inputs.
    Data(Vec<(String, String)>),
}

impl Dataset {
    #[must_use]
    pub fn is_generated(&self) -> bool {
        matches!(self, Self::Generated(..))
    }
}

/// An alignment job: a single task for the runner to execute and benchmark.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Job {
    /// Path to a `.seq` file.
    pub dataset: Dataset,
    /// The cost model to use.
    pub costs: CostModel,
    /// Return the full alignment/cigar?
    pub traceback: bool,
    /// The algorithm/parameters to use.
    pub algo: AlgorithmParams,
}

impl Job {
    /// Whether this job is larger than another job.
    /// Returns false when either job is not generated.
    pub fn is_larger(&self, o: &Self) -> bool {
        let Dataset::Generated(self_args) = &self.dataset else {
            return false;
        };
        let Dataset::Generated(o_args) = &o.dataset else {
            return false;
        };
        self.costs == o.costs
            && self.algo == o.algo
            && self.traceback == o.traceback
            && self_args.is_larger_than(o_args)
    }

    pub fn same_input(&self, o: &Self) -> bool {
        self.dataset == o.dataset && self.costs == o.costs
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct Measured {
    /// Runtime in seconds.
    pub runtime: f32,
    pub memory: Bytes,
    pub cpu_freq_start: Option<f32>,
    pub cpu_freq_end: Option<f32>,
    pub cpu_clocks: Option<u64>,
}

/// The output of an alignment job.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobOutput {
    pub costs: Vec<Cost>,
    /// Corresponding exact costs if the job is approximate.
    pub exact_costs: Option<Vec<Cost>>,
    //pub cigars: Vec<Cigar>,
    pub exact: bool,
    /// Proportion of correct costs.
    pub p_correct: Option<f32>,
    pub measured: Measured,
}

/// The result of an alignment job, containing the input and output.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JobResult {
    pub job: Job,
    // TODO(ragnar): Make this a result with a specific error type that indicates the failure reason.
    pub output: Result<JobOutput, ()>,
}
