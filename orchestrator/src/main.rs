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

mod generator;
use generator::*;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to a yaml file with a list of parameters.
    jobs: PathBuf,

    /// Path to the output json file.
    results: PathBuf,

    /// Path to the data directory.
    #[arg(short, long, default_value = "data")]
    data_dir: PathBuf,

    /// Path to the runner binary.
    #[arg(short, long, default_value = "target/release/runner")]
    runner: PathBuf,

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

    /// Force rerun dataset generation and all jobs.
    ///
    /// Helpful for making sure all files are up to date.
    #[arg(long)]
    force_rerun: bool,
}

fn main() {
    let args = Args::parse();

    assert!(
        args.runner.exists(),
        "{} does not exist!",
        args.runner.display()
    );

    let jobs_yaml = fs::read_to_string(&args.jobs)
        .map_err(|err| format!("Failed to read jobs generator: {err}"))
        .unwrap();
    let jobs_config: JobsConfig = serde_yaml::from_str(&jobs_yaml)
        .map_err(|err| format!("Failed to parse jobs generator yaml: {err}"))
        .unwrap();

    // Read the existing results file.
    let mut existing_job_results: Vec<JobResult> = if !args.force_rerun && args.results.is_file() {
        serde_json::from_str(&fs::read_to_string(&args.results).unwrap()).unwrap()
    } else {
        vec![]
    };

    eprintln!("There are {} existing jobs!", existing_job_results.len());
    eprintln!("Generating jobs and datasets...");
    let jobs = jobs_config.generate(&args.data_dir, args.force_rerun);
    eprintln!("Generated {} jobs!", jobs.len());
    // Remove jobs that were run before.
    let jobs = if args.incremental && !args.force_rerun {
        jobs.into_iter()
            .filter(|job| {
                existing_job_results
                    .iter()
                    .find(|existing_job| &existing_job.job == job)
                    .is_none()
            })
            .collect()
    } else {
        jobs
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
        &args.runner,
        jobs,
        args.time_limit,
        args.mem_limit,
        runner_cores,
        args.nice,
        args.stderr,
        args.verbose,
    );

    if !args.force_rerun {
        // Remove jobs that were run from existing results.
        existing_job_results = existing_job_results
            .into_iter()
            .filter(|existing_job| {
                job_results
                    .iter()
                    .find(|job| job.job == existing_job.job)
                    .is_none()
            })
            .collect();
    }
    let job_results_len = job_results.len();
    // Append new results to existing results.
    existing_job_results.extend(job_results);
    eprintln!(
        "Finished running {} jobs! Totaling {} job results.",
        job_results_len,
        existing_job_results.len()
    );

    let job_results = verify_costs(existing_job_results);

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
fn verify_costs(mut results: Vec<JobResult>) -> Vec<JobResult> {
    // Ensure exact algorithms are first in results.
    results.sort_by_key(|res| !res.output.as_ref().map(|o| o.exact).unwrap_or(false));

    results.iter().enumerate().map(|(i, result)| {
        if result.output.is_err() {
            return result.clone();
        }
        let output = result.output.as_ref().unwrap();

        // Find the first job with the same input and compare costs.
        for result2 in &results[..i] {
            if result2.output.is_ok() && result2.job.same_input(&result.job) {
                let output2 = result2.output.as_ref().unwrap();

                if output2.exact {
                    if output.exact {
                        assert_eq!(
                            output.costs,
                            output2.costs,
                            "\nIncorrect costs of exact algorithms!\nJob 1: {:?}\nJob 2: {:?}\nCosts 1: {:?}\nCosts 2: {:?}",
                            result.job,
                            result2.job,
                            output.costs,
                            output2.costs,
                        );
                        return result.clone();
                    } else {
                        let mut res = result.clone();
                        res.output.as_mut().unwrap().exact_costs = Some(output2.costs.clone());
                        let num_correct = output.costs.iter().zip(&output2.costs).filter(|(&a, &b)| a == b).count();
                        res.output.as_mut().unwrap().p_correct = (num_correct as f32) / (output.costs.len() as f32);
                        return res;
                    }
                }
            }
        }

        result.clone()
    }).collect()
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
            output: Ok(serde_json::from_slice(&output.stdout)
                .map_err(|err| format!("Error reading output json: {err}"))
                .unwrap()),
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
