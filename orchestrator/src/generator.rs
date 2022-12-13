use serde::{Deserialize, Serialize};

use std::fs;
use std::path::{Path, PathBuf};

use itertools::iproduct;

use pa_generate::*;
use pa_types::*;
use pa_bench_types::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct JobsGenerator {
    datasets: Vec<Dataset>,
    traces: Vec<bool>,
    costs: Vec<CostModel>,
    algos: Vec<AlgorithmParams>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Dataset {
    Generate(DataGenerator),
    File(PathBuf),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataGenerator {
    prefix: String,
    seed: u64,
    error_models: Vec<ErrorModel>,
    error_rates: Vec<f32>,
    lengths: Vec<usize>,
    total_size: usize,
}

impl JobsGenerator {
    pub fn generate(self, data_dir: &Path) -> Vec<Job> {
        let datasets = self.datasets.into_iter().map(|d| d.generate(data_dir).into_iter()).flatten();
        iproduct!(datasets, self.costs, self.traces, self.algos)
            .map(|(dataset, costs, traceback, algo)| Job {
                dataset,
                costs,
                traceback,
                algo,
            })
        .collect()
    }
}

impl Dataset {
    pub fn generate(self, data_dir: &Path) -> Vec<PathBuf> {
        match self {
            Dataset::Generate(generator) => generator.generate(data_dir),
            Dataset::File(path) => vec![path.clone()],
        }
    }
}

impl DataGenerator {
    /// Generates missing `.seq` files in a directory and returns them.
    pub fn generate(self, data_dir: &Path) -> Vec<PathBuf> {
        let dir = data_dir.join(&self.prefix);
        fs::create_dir_all(&dir).unwrap();

        iproduct!(self.error_models, self.error_rates, self.lengths)
            .map(|(error_model, error_rate, length)| {
                let path = dir.join(format!("{error_model:?}-n{length}-e{error_rate}.seq"));
                GenerateArgs {
                    options: GenerateOptions {
                        length,
                        error_rate,
                        error_model,
                        pattern_length: None,
                    },
                    seed: Some(self.seed),
                    cnt: None,
                    size: Some(self.total_size),
                }
                .generate_file(&path);
                path
            })
        .collect()
    }
}