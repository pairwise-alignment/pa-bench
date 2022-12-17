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
    Data(Vec<(String, String)>),
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
    pub fn generate(self, data_dir: &Path, force_rerun: bool) -> Vec<Job> {
        let datasets = self
            .datasets
            .into_iter()
            .flat_map(|d| d.generate(data_dir, force_rerun).into_iter());
        iproduct!(datasets, self.costs, self.traces, self.algos)
            .map(|((dataset, meta), costs, traceback, algo)| Job {
                dataset,
                costs,
                traceback,
                algo,
                meta,
            })
            .collect()
    }
}

impl Dataset {
    pub fn generate(self, data_dir: &Path, force_rerun: bool) -> Vec<(PathBuf, Option<DatasetMetadata>)> {
        match self {
            Dataset::Generate(generator) => generator.generate(data_dir, force_rerun),
            Dataset::File(path) => vec![(path.clone(), None)],
            Dataset::Data(data) => {
                let mut state = DefaultHasher::new();
                data.hash(&mut state);
                let path = data_dir.join(format!("manual/{}.seq", state.finish()));
                std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
                let mut f = BufWriter::new(std::fs::File::create(&path).unwrap());
                for (a, b) in data {
                    writeln!(f, ">{a}").unwrap();
                    writeln!(f, "<{b}").unwrap();
                }
                vec![(path, None)]
            }
        }
    }
}

impl DataGenerator {
    /// Generates missing `.seq` files in a directory and returns them.
    pub fn generate(self, data_dir: &Path, force_rerun: bool) -> Vec<(PathBuf, Option<DatasetMetadata>)> {
        let dir = data_dir.join(&self.prefix);
        fs::create_dir_all(&dir).unwrap();

        iproduct!(self.error_models, self.error_rates, self.lengths)
            .map(|(error_model, error_rate, length)| {
                let path = dir.join(format!(
                    "{error_model:?}-t{}-n{length}-e{error_rate}.seq",
                    self.total_size
                ));
                if force_rerun || !path.exists() {
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
                }
                (
                    path,
                    Some(DatasetMetadata {
                        error_model,
                        error_rate,
                        length,
                    }),
                )
            })
            .collect()
    }
}
