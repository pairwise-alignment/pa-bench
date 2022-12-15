use pa_bench_types::*;
use pa_types::*;

use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
    let generator: JobsGenerator = serde_yaml::from_str(&jobs_yaml)
        .map_err(|err| format!("Failed to parse jobs generator yaml: {err}"))
        .unwrap();
    let mut jobs = generator.generate(&args.data_dir);
    // Filter out jobs already present.
    let mut job_results = if args.incremental && args.results.is_file() {
        let existing_results: Vec<JobResult> =
            serde_json::from_str(&fs::read_to_string(&args.results).unwrap()).unwrap();
        jobs = jobs
            .into_iter()
            .filter(|job| {
                existing_results
                    .iter()
                    .find(|existing_job| &existing_job.job == job)
                    .is_none()
            })
            .collect();
        existing_results
    } else {
        vec![]
    };

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
    job_results.extend(run_with_threads(
        &args.runner,
        jobs,
        args.time_limit,
        args.mem_limit,
        runner_cores,
        args.nice,
        args.stderr,
    ));

    if let Some(dir) = args.results.parent() {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(&args.results, &serde_json::to_string(&job_results).unwrap()).expect(&format!(
        "Failed to write results to {}",
        args.results.display()
    ));
}

fn run_with_threads(
    runner: &Path,
    jobs: Vec<Job>,
    time_limit: Duration,
    mem_limit: Bytes,
    cores: Option<Vec<usize>>,
    nice: Option<i32>,
    show_stderr: bool,
) -> Vec<JobResult> {
    let job_results = Mutex::new(Vec::<JobResult>::with_capacity(jobs.len()));
    let jobs_iter = Mutex::new(jobs.into_iter());

    // Make a `Vec<Option<usize>>` which defaults to `[None]`.
    let cores = cores
        .map(|cores| cores.into_iter().map(Some).collect())
        .unwrap_or(vec![None]);

    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || {
            eprintln!("Pressed Ctrl-C. Stopping running jobs.");
            r.store(false, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
    }

    thread::scope(|scope| {
        for id in &cores {
            scope.spawn(|| {
                while let Some(job) = jobs_iter.lock().unwrap().next() {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    // If a smaller job for the same algorithm failed, skip it.
                    let mut skip = false;
                    if job.meta.is_some() {
                        for prev in job_results.lock().unwrap().iter() {
                            if prev.output.is_none()
                                && prev.job.meta.is_some()
                                && job.is_larger(&prev.job)
                            {
                                skip = true;
                                break;
                            }
                        }
                    }

                    let job_result = if skip {
                        JobResult { job, output: None }
                    } else {
                        run_job(runner, job, time_limit, mem_limit, *id, nice, show_stderr)
                    };

                    job_results.lock().unwrap().push(job_result);
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
            output: Some(
                serde_json::from_slice(&output.stdout)
                    .map_err(|err| format!("Error reading output json: {err}"))
                    .unwrap(),
            ),
        }
    } else {
        JobResult { job, output: None }
    }
}
