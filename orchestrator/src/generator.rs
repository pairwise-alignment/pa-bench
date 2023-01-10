use serde::{Deserialize, Serialize};

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use itertools::iproduct;

use pa_bench_types::*;
use pa_generate::*;
use pa_types::*;

/// The main configuration object and root of the yaml file.
#[derive(Serialize, Deserialize, Debug)]
pub struct JobsConfig {
    datasets: Vec<DatasetConfig>,
    traces: Vec<bool>,
    costs: Vec<CostModel>,
    algos: Vec<AlgorithmParams>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DatasetConfig {
    Generated(DatasetGeneratorConfig),
    File(PathBuf),
    Data(Vec<(String, String)>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatasetGeneratorConfig {
    prefix: String,
    seed: u64,
    error_models: Vec<ErrorModel>,
    error_rates: Vec<f32>,
    lengths: Vec<usize>,
    total_size: usize,
}

impl JobsConfig {
    pub fn generate(self, data_dir: &Path, force_rerun: bool) -> Vec<Job> {
        let datasets = self
            .datasets
            .into_iter()
            .flat_map(|d| d.generate(data_dir, force_rerun).into_iter());
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

impl DatasetConfig {
    pub fn generate(self, data_dir: &Path, force_rerun: bool) -> Vec<Dataset> {
        match self {
            DatasetConfig::Generated(generator) => generator.generate(data_dir, force_rerun),
            DatasetConfig::File(path) => vec![Dataset::File(path.clone())],
            DatasetConfig::Data(data) => {
                let mut state = DefaultHasher::new();
                data.hash(&mut state);
                let path = data_dir.join(format!("manual/{}.seq", state.finish()));
                std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
                let mut f = BufWriter::new(std::fs::File::create(&path).unwrap());
                for (a, b) in data {
                    writeln!(f, ">{a}").unwrap();
                    writeln!(f, "<{b}").unwrap();
                }
                vec![Dataset::File(path)]
            }
        }
    }
}

impl DatasetGeneratorConfig {
    /// Generates missing `.seq` files in a directory and returns them.
    pub fn generate(self, data_dir: &Path, force_rerun: bool) -> Vec<Dataset> {
        let dir = data_dir.join(&self.prefix);
        fs::create_dir_all(&dir).unwrap();

        iproduct!(self.error_models, self.error_rates, self.lengths)
            .map(|(error_model, error_rate, length)| {
                let generated_dataset = GeneratedDataset {
                    prefix: dir.clone(),
                    seed: self.seed,
                    error_model,
                    error_rate,
                    length,
                    total_size: self.total_size,
                    pattern_length: None,
                };
                let path = generated_dataset.path();
                if force_rerun || !path.exists() {
                    generated_dataset.to_generator().generate_file(&path);
                }
                Dataset::Generated(generated_dataset)
            })
            .collect()
    }
}
