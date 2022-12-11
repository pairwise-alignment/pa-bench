use pa_bench_types::*;
use pa_types::*;

use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json;

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

    let job_results: Vec<JobResult> = jobs
        .into_iter()
        .map(|job| run(&args.runner, job, args.time_limit, args.mem_limit))
        .collect();

    fs::write(&args.results, &serde_json::to_string(&job_results).unwrap()).expect(&format!(
        "Failed to write results to {}",
        args.results.display()
    ));
}

fn run(runner: &Path, job: Job, time_limit: Duration, mem_limit: Bytes) -> JobResult {
    let child = Command::new(runner)
        .arg("--time-limit")
        .arg(time_limit.as_secs().to_string())
        .arg("--mem-limit")
        .arg(mem_limit.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let mut stdin = child.stdin.as_ref().unwrap();
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
