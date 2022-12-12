use pa_bench_types::*;
use pa_types::*;

use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde_json;

use core_affinity;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to a json file with a list of parameters.
    jobs: PathBuf,

    /// Path to the output json file.
    results: PathBuf,

    /// Path to the runner binary.
    #[arg(short, long, default_value = "target/release/runner")]
    runner: PathBuf,

    #[arg(short, long, value_parser = parse_duration::parse, default_value = "1h")]
    time_limit: Duration,

    #[arg(short, long, value_parser = parse_bytes, default_value = "1GiB")]
    mem_limit: Bytes,

    /// Number of processes to use.
    ///
    /// Processes are pinned to separate cores.
    /// The number of processes is capped to the total number of cores.
    #[arg(short, long)]
    processes: Option<usize>,
}

fn main() {
    let args = Args::parse();
    let jobs_json = fs::read_to_string(&args.jobs)
        .map_err(|err| format!("Failed to read jobs: {err}"))
        .unwrap();
    let jobs: Vec<Job> = serde_json::from_str(&jobs_json)
        .map_err(|err| format!("Failed to parse jobs json: {err}"))
        .unwrap();

    assert!(
        args.runner.exists(),
        "{} does not exist!",
        args.runner.display()
    );

    let job_results: Vec<JobResult> = if let Some(processes) = args.processes {
        let pin_cores = core_affinity::get_core_ids()
            .unwrap()
            .into_iter()
            .take(processes)
            .map(|c| c.id)
            .collect();
        run_with_threads(
            &args.runner,
            jobs,
            args.time_limit,
            args.mem_limit,
            pin_cores,
        )
    } else {
        jobs.into_iter()
            .map(|job| run(&args.runner, job, args.time_limit, args.mem_limit, None))
            .collect()
    };

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
    cores: Vec<usize>,
) -> Vec<JobResult> {
    let jobs_iter = Mutex::new(jobs.into_iter());
    let job_results = Mutex::new(Vec::new());

    thread::scope(|scope| {
        for id in &cores {
            scope.spawn(|| loop {
                if let Some(job) = jobs_iter.lock().unwrap().next() {
                    let job_result = run(runner, job, time_limit, mem_limit, Some(*id));
                    job_results.lock().unwrap().push(job_result);
                } else {
                    break;
                }
            });
        }
    });

    job_results.into_inner().unwrap()
}

fn run(
    runner: &Path,
    job: Job,
    time_limit: Duration,
    mem_limit: Bytes,
    core_id: Option<usize>,
) -> JobResult {
    let mut cmd = Command::new(runner);
    cmd.arg("--time-limit")
        .arg(time_limit.as_secs().to_string())
        .arg("--mem-limit")
        .arg(mem_limit.to_string());
    if let Some(id) = core_id {
        cmd.arg("--pin-core-id").arg(id.to_string());
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

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
