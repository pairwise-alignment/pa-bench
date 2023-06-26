use flate2::bufread::GzDecoder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
#[rustfmt::skip]
use ::stats::merge_all;
use tar::Archive;
use walkdir::DirEntry;

use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use itertools::{iproduct, Itertools};

use fxhash::FxHasher;

use pa_generate::*;
use pa_types::*;

use crate::stats::file_stats;
use crate::*;

/// This is the root type of the yaml configuration file.
/// It consists of multiple Experiments, each of which is the Cartesian product
/// of a set of datasets and parameters.
#[derive(Serialize, Deserialize, Debug)]
pub struct Experiments(Vec<Experiment>);

/// A SingleExperiment runs each algo for each cost model with each trace on
/// each of the specified datasets.
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Experiment {
    /// A comment about the dataset
    comment: Option<String>,
    /// Parsed using parse_duration::parse.
    /// Default: 1m.
    /// Can be overridden by command line flag.
    time_limit: Option<String>,
    /// Parsed using parse_bytes.
    /// Default: 1GiB.
    /// Can be overridden by command line flag.
    mem_limit: Option<String>,
    datasets: Vec<DatasetConfig>,
    traces: Vec<bool>,
    costs: Vec<CostModel>,
    algos: Vec<AlignerParams>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub enum DatasetConfig {
    Generated(DatasetGeneratorConfig),
    /// A file, or all .seq files in the given directory, relative to `--data-dir`.
    Path(PathBuf),
    /// Download `url`, and extract to `dir` relative to `--data-dir`.
    /// `url` must end in either `.zip` or `.tar.gz`.
    Download {
        url: String,
        dir: PathBuf,
    },
    /// The data itself.
    Data(Vec<(String, String)>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DatasetGeneratorConfig {
    seed: u64,
    error_models: Vec<ErrorModel>,
    error_rates: Vec<f32>,
    lengths: Vec<usize>,
    total_size: Option<usize>,
    count: Option<usize>,
}

impl Experiments {
    pub fn generate(
        self,
        data_dir: &Path,
        regenerate: bool,
        time_limit: Option<Duration>,
        mem_limit: Option<Bytes>,
    ) -> Vec<(Job, DatasetStats)> {
        self.0
            .into_iter()
            .flat_map(|product| {
                let time_limit = time_limit
                    .unwrap_or(
                        // 1 year 'infinite' time limit by default.
                        parse_duration::parse(&product.time_limit.unwrap_or("1y".into()))
                            .expect("Could not parse time limit"),
                    )
                    .as_secs();
                let mem_limit = mem_limit.unwrap_or(
                    // 1000 TiB 'infinite' memory limit by default.
                    parse_bytes(&product.mem_limit.unwrap_or("1000TiB".into()))
                        .expect("Could not parse memory limit"),
                );
                let datasets = product
                    .datasets
                    .into_iter()
                    .flat_map(|d| d.generate(data_dir, regenerate).into_iter())
                    .collect_vec();
                iproduct!(datasets, product.costs, product.traces, product.algos).map(
                    move |((dataset, stats), costs, traceback, algo)| {
                        (
                            Job {
                                time_limit,
                                mem_limit,
                                dataset,
                                costs,
                                traceback,
                                algo,
                            },
                            stats,
                        )
                    },
                )
            })
            .collect()
    }
}

impl DatasetConfig {
    pub fn generate(self, data_dir: &Path, regenerate: bool) -> Vec<(Dataset, DatasetStats)> {
        fn collect_dir(dir: &Path) -> Vec<Dataset> {
            assert!(dir.is_dir() && dir.exists());
            fn is_hidden(entry: &DirEntry) -> bool {
                entry
                    .file_name()
                    .to_str()
                    .map(|s| s.starts_with("."))
                    .unwrap_or(false)
            }
            walkdir::WalkDir::new(dir)
                .into_iter()
                .filter_entry(|e| !is_hidden(e))
                .filter_map(|e| {
                    let e = e.unwrap();
                    if e.file_type().is_file()
                        && e.path().extension().map_or(false, |ext| ext == "seq")
                    {
                        Some(e.path().to_path_buf())
                    } else {
                        None
                    }
                })
                .sorted()
                .map(|file| Dataset::File(file))
                .collect()
        }

        let (dir_stats_path, dataset) = match self {
            DatasetConfig::Generated(generator) => (None, generator.generate(data_dir, regenerate)),
            DatasetConfig::Path(path) => {
                let path = data_dir.join(&path);
                if path.is_dir() {
                    (Some(path.join("stats.json")), collect_dir(&path))
                } else {
                    (None, vec![Dataset::File(path)])
                }
            }
            DatasetConfig::Download { url, dir } => {
                let target_dir = &data_dir.join("download").join(&dir);
                let dir_empty = target_dir
                    .read_dir()
                    .map_or(true, |mut d| d.next().is_none());
                if dir_empty {
                    fs::create_dir_all(target_dir).unwrap();
                    // download the url
                    let mut data = vec![];
                    eprintln!("Downloading {}: {url}", dir.display());
                    ureq::get(&url)
                        .call()
                        .unwrap()
                        .into_reader()
                        .read_to_end(&mut data)
                        .unwrap();
                    eprintln!("Extracting to {}", target_dir.display());
                    match url {
                        url if url.ends_with(".zip") => {
                            zip_extract::extract(Cursor::new(data), target_dir, true).unwrap()
                        }
                        url if url.ends_with(".tar.gz") => {
                            let tar = GzDecoder::new(Cursor::new(data));
                            let mut archive = Archive::new(tar);
                            archive.unpack(target_dir).unwrap();
                        }
                        _ => panic!("Download url must end in .zip or .tar.gz."),
                    };
                }
                (Some(target_dir.join("stats.json")), collect_dir(target_dir))
            }
            DatasetConfig::Data(data) => {
                let hash = {
                    // Use deterministic hasher for file names!
                    let mut state = FxHasher::default();
                    data.hash(&mut state);
                    state.finish()
                };
                let path = data_dir.join(format!("manual/{hash}.seq"));
                std::fs::create_dir_all(&path.parent().unwrap()).unwrap();
                let mut f = BufWriter::new(std::fs::File::create(&path).unwrap());
                for (a, b) in data {
                    writeln!(f, ">{a}").unwrap();
                    writeln!(f, "<{b}").unwrap();
                }
                (None, vec![Dataset::File(path)])
            }
        };

        // Collect stats for each file, and collect summary stats for Directory and Download rules.
        // For directories and downloads, merged summary stats are only generated on the initial download.
        // Either way, missing per-file stats are always generated.

        let datasets_with_stats = dataset
            .into_par_iter()
            .map(|d| {
                let f = match &d {
                    Dataset::Generated(g) => g.path(),
                    Dataset::File(f) => f.to_path_buf(),
                    // TODO: Generate stats on the fly.
                    Dataset::Data(_) => todo!(),
                };
                let stats_path = f.with_extension("stats.json");
                let stats = if stats_path.is_file() {
                    let v = fs::read(&stats_path).unwrap();
                    serde_json::from_slice(&v)
                        .expect(&format!("Could not parse {} as json", stats_path.display()))
                } else {
                    let stats = file_stats(&f);
                    fs::write(&stats_path, serde_json::to_string_pretty(&stats).unwrap())
                        .expect("Failed to write to stats file!");
                    stats
                };
                (d, stats)
            })
            .collect::<Vec<_>>();
        let merged_stats = merge_all(datasets_with_stats.iter().map(|(_d, s)| s.clone()));
        if let Some(stats_path) = dir_stats_path {
            if let Some(merged_stats) = merged_stats {
                if !stats_path.exists() {
                    eprintln!("Write summary stats to {}", stats_path.display());
                    fs::create_dir_all(&stats_path.parent().unwrap()).unwrap();
                    fs::write(
                        &stats_path,
                        serde_json::to_string_pretty(&merged_stats).unwrap(),
                    )
                    .expect("Failed to write to stats file!");
                }
            }
        }

        datasets_with_stats
    }
}

impl DatasetGeneratorConfig {
    /// Generates missing `.seq` files in a directory and returns them.
    pub fn generate(self, data_dir: &Path, regenerate: bool) -> Vec<Dataset> {
        let dir = data_dir.join("generated");
        fs::create_dir_all(&dir).unwrap();

        iproduct!(self.error_models, self.error_rates, self.lengths)
            .map(|(error_model, error_rate, length)| {
                let generated_dataset = GeneratedDataset {
                    prefix: dir.clone(),
                    seed: self.seed,
                    error_model,
                    error_rate,
                    length,
                    total_size: self.total_size.unwrap_or_else(|| {
                        self.count.expect("total_size or count must be set") * length
                    }),
                    pattern_length: None,
                };
                let path = generated_dataset.path();
                if regenerate || !(path.exists() && path.metadata().unwrap().len() > 0) {
                    eprintln!("Generating dataset {}", path.display());
                    generated_dataset.to_generator().generate_file(&path);
                }
                Dataset::Generated(generated_dataset)
            })
            .collect()
    }
}
