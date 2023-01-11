use chrono::Timelike;
use pa_bench_types::*;
use pa_types::*;

use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use std::os::unix::process::ExitStatusExt;

use serde_json;
use serde_yaml;

use core_affinity;

use clap::Parser;

mod config;
use config::*;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to an experiment yaml file.
    experiment: PathBuf,

    /// Path to the output json file.
    #[arg(default_value = "evals/results.json")]
    results: PathBuf,

    /// Path to the data directory.
    #[arg(short, long, default_value = "evals/data")]
    data_dir: PathBuf,

    /// Path to the logs directory.
    #[arg(short, long, default_value = "evals/.results")]
    logs_dir: PathBuf,

    /// Path to the runner binary. Uses $CARGO_MANIFEST_DIR/../target/release/runner by default.
    #[arg(short, long)]
    runner: Option<PathBuf>,

    #[arg(short, long, value_parser = parse_duration::parse, default_value = "1h")]
    time_limit: Duration,

    #[arg(short, long, value_parser = parse_bytes, default_value = "1GiB")]
    mem_limit: Bytes,

    // process niceness. <0 for higher priority.
    #[arg(long)]
    nice: Option<i32>,

    /// Number of parallel jobs to use.
    ///
    /// Jobs are pinned to separate cores.
    /// The number of jobs is capped to the total number of cores minus 1.
    #[arg(short = 'j', long)]
    num_jobs: Option<usize>,

    /// Show stderr of runner process.
    #[arg(long)]
    stderr: bool,

    /// Skip jobs already present in the results file.
    #[arg(long)]
    incremental: bool,

    /// Verbose runner outputs.
    #[arg(short, long)]
    verbose: bool,

    /// Ignore the existing results json and regenerate datasets.
    #[arg(long)]
    force_rerun: bool,
}

fn main() {
    let mut args = Args::parse();
    if args.runner.is_none() {
        let dir = std::env::var("CARGO_MANIFEST_DIR")
            .expect("Neither --runner nor CARGO_MANIFEST_DIR env var is set.");
        args.runner = Some(Path::new(&dir).join("../target/release/runner"));
    }

    assert!(
        args.runner.as_ref().unwrap().exists(),
        "{} does not exist!",
        args.runner.unwrap().display()
    );

    let experiment_yaml =
        fs::read_to_string(&args.experiment).expect("Failed to read jobs generator:");
    let experiment: Experiment =
        serde_yaml::from_str(&experiment_yaml).expect("Failed to parse jobs generator yaml:");

    // Read the existing results file.
    let mut existing_job_results: Vec<JobResult> = if !args.force_rerun && args.results.is_file() {
        serde_json::from_str(
            &fs::read_to_string(&args.results).expect("Error reading existing results file"),
        )
        .expect("Error parsing existing results")
    } else {
        vec![]
    };

    eprintln!("There are {} existing jobs!", existing_job_results.len());
    eprintln!("Generating jobs and datasets...");
    let mut jobs = experiment.generate(&args.data_dir, args.force_rerun);
    eprintln!("Generated {} jobs!", jobs.len());
    // Remove jobs that were run before.
    if args.incremental {
        jobs.retain(|job| {
            existing_job_results
                .iter()
                .find(|existing_job| &existing_job.job == job)
                .is_none()
        });
    };
    eprintln!("Running {} jobs...", jobs.len());

    let runner_cores = if let Some(num_jobs) = args.num_jobs {
        let mut cores = core_affinity::get_core_ids()
            .unwrap()
            .into_iter()
            // NOTE: This assumes that virtual cores 0 and n/2 are on the same
            // physical core, in case hyperthreading is enabled.
            // TODO(ragnar): Is it better to spread the load over non-adjacent
            // physical cores? Unclear to me.
            .take(num_jobs + 1);

        // Reserve one core for the orchestrator.
        let orchestrator_core = cores.next().unwrap();
        core_affinity::set_for_current(orchestrator_core);

        // Remaining (up to) #processes cores are for runners.
        Some(cores.map(|c| c.id).collect())
    } else {
        None
    };

    let job_results = run_with_threads(
        &args.runner.unwrap(),
        jobs,
        args.time_limit,
        args.mem_limit,
        runner_cores,
        args.nice,
        args.stderr,
        args.verbose,
    );
    let number_of_jobs_run = job_results.len();

    {
        let logs_path = args.logs_dir.join(format!(
            "{}_{}.json",
            args.experiment.file_stem().unwrap().to_str().unwrap(),
            chrono::Local::now()
                .with_nanosecond(0)
                .unwrap()
                .to_rfc3339()
        ));
        // Write results to persistent log.
        fs::create_dir_all(args.logs_dir).unwrap();
        fs::write(&logs_path, &serde_json::to_string(&job_results).unwrap())
            .expect(&format!("Failed to write logs to {}", logs_path.display()));
    }

    // Remove jobs that were run from existing results.
    existing_job_results.retain(|existing_job| {
        job_results
            .iter()
            .find(|job| job.job == existing_job.job)
            .is_none()
    });

    // Append new results to existing results.
    existing_job_results.extend(job_results);
    let mut job_results = existing_job_results;

    eprintln!(
        "Finished running {} jobs! Totalling {} job results.",
        number_of_jobs_run,
        job_results.len()
    );

    verify_costs(&mut job_results);

    if let Some(dir) = args.results.parent() {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(&args.results, &serde_json::to_string(&job_results).unwrap()).expect(&format!(
        "Failed to write results to {}",
        args.results.display()
    ));

    println!("Orchestrator successfully finished!");
}

/// Verify costs for exact algorithms and count correct costs for approximate algorithms.
fn verify_costs(results: &mut Vec<JobResult>) {
    // Ensure exact algorithms are first in results.
    results.sort_by_key(|res| !res.output.as_ref().map(|o| o.is_exact).unwrap_or(false));

    for i in 0..results.len() {
        let (earlier_results, result) = results.split_at_mut(i);
        let result = &mut result[0];

        let Ok(output) = result.output.as_mut() else {
            // Nothing to do for failed jobs.
            continue;
        };

        // Find the first exact job with the same input and compare costs.
        for reference_result in earlier_results {
            if !reference_result.job.same_input(&result.job) {
                continue;
            }
            let Ok(reference_output) = reference_result.output.as_ref() else {
                continue;
            };
            if !reference_output.is_exact {
                continue;
            }
            assert_eq!(
                output.costs.len(),
                reference_output.costs.len(),
                "\nDifferent number of costs!\nJob 1: {:?}\nJob 2: {:?}\nLen costs 1: {:?}\nLen costs 2: {:?}",
                result.job,
                reference_result.job,
                output.costs.len(),
                reference_output.costs.len(),
            );
            if output.is_exact {
                // For exact jobs, simply check they give the same result.
                assert_eq!(
                            output.costs,
                            reference_output.costs,
                            "\nIncorrect costs of exact algorithms!\nJob 1: {:?}\nJob 2: {:?}\nCosts 1: {:?}\nCosts 2: {:?}",
                            result.job,
                            reference_result.job,
                            output.costs,
                            reference_output.costs,
                        );
            } else {
                // For inexact jobs, add the correct ones and the fraction of correct results.
                output.exact_costs = Some(reference_output.costs.clone());
                let num_correct = output
                    .costs
                    .iter()
                    .zip(&reference_output.costs)
                    .filter(|(&a, &b)| a == b)
                    .count();
                output.p_correct = Some((num_correct as f32) / (output.costs.len() as f32));
            }
        }
    }
}

fn run_with_threads(
    runner: &Path,
    jobs: Vec<Job>,
    time_limit: Duration,
    mem_limit: Bytes,
    cores: Option<Vec<usize>>,
    nice: Option<i32>,
    show_stderr: bool,
    verbose: bool,
) -> Vec<JobResult> {
    let job_results = Mutex::new(Vec::<JobResult>::with_capacity(jobs.len()));
    let jobs_iter = Mutex::new(jobs.into_iter());

    // Make a `Vec<Option<usize>>` which defaults to `[None]`.
    let cores = cores
        .map(|cores| cores.into_iter().map(Some).collect())
        .unwrap_or(vec![None]);

    let running = Arc::new(Mutex::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || {
            eprintln!("Pressed Ctrl-C. Stopping running jobs.");
            *r.lock().unwrap() = false;
        })
        .expect("Error setting Ctrl-C handler");
    }

    thread::scope(|scope| {
        for id in &cores {
            scope.spawn(|| {
                loop {
                    let Some(job) = jobs_iter.lock().unwrap().next() else {
                        break;
                    };
                    if !*running.lock().unwrap() {
                        break;
                    }
                    // If a smaller job for the same algorithm failed, skip it.
                    let mut skip = false;
                    if job.dataset.is_generated() {
                        for prev in job_results.lock().unwrap().iter() {
                            if prev.output.is_err()
                                && prev.job.dataset.is_generated()
                                && job.is_larger(&prev.job)
                            {
                                skip = true;
                                break;
                            }
                        }
                    }

                    let job_result = if skip {
                        JobResult {
                            job,
                            output: Err(()),
                        }
                    } else {
                        run_job(
                            runner,
                            job,
                            time_limit,
                            mem_limit,
                            *id,
                            nice,
                            show_stderr,
                            verbose,
                        )
                    };

                    // If the orchestrator was aborted, do not push failing job results.
                    if job_result.output.is_ok() || *running.lock().unwrap() {
                        job_results.lock().unwrap().push(job_result);
                    }
                }
            });
        }
    });

    job_results.into_inner().unwrap()
}

fn run_job(
    runner: &Path,
    job: Job,
    time_limit: Duration,
    mem_limit: Bytes,
    core_id: Option<usize>,
    nice: Option<i32>,
    show_stderr: bool,
    verbose: bool,
) -> JobResult {
    let mut cmd = Command::new(runner);
    cmd.arg("--time-limit")
        .arg(time_limit.as_secs().to_string())
        .arg("--mem-limit")
        .arg(mem_limit.to_string());
    if let Some(id) = core_id {
        cmd.arg("--pin-core-id").arg(id.to_string());
    }
    if let Some(nice) = nice {
        // negative numbers need to be passed with =.
        cmd.arg(format!("--nice={nice}"));
    }
    if verbose {
        cmd.arg("--verbose");
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    if !show_stderr {
        cmd.stderr(Stdio::null());
    }
    let mut child = cmd.spawn().unwrap();

    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(&serde_json::to_vec(&job).unwrap()).unwrap();
    }

    let output = child.wait_with_output().unwrap();

    if output.status.success() {
        JobResult {
            job,
            output: Ok(serde_json::from_slice(&output.stdout).expect("Error reading output json:")),
        }
    } else {
        if show_stderr {
            if let Some(code) = output.status.signal() {
                if code == 24 {
                    eprintln!("Time limit exceeded for {job:?}");
                }
            }
        }
        JobResult {
            job,
            output: Err(()),
        }
    }
}
