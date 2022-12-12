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

    /// Number of parallel jobs to use.
    ///
    /// Jobs are pinned to separate cores.
    /// The number of jobs is capped to the total number of cores minus 1.
    #[arg(short = 'j', long)]
    num_jobs: Option<usize>,

    /// Show stderr of runner process
    #[arg(long)]
    stderr: bool,
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

    let job_results: Vec<JobResult> = if let Some(num_jobs) = args.num_jobs {
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
        let runner_cores = cores.map(|c| c.id).collect();
        run_with_threads(
            &args.runner,
            jobs,
            args.time_limit,
            args.mem_limit,
            runner_cores,
            args.stderr,
        )
    } else {
        jobs.into_iter()
            .map(|job| {
                run(
                    &args.runner,
                    job,
                    args.time_limit,
                    args.mem_limit,
                    None,
                    args.stderr,
                )
            })
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
    show_stderr: bool,
) -> Vec<JobResult> {
    let job_results = Mutex::new(Vec::with_capacity(jobs.len()));
    let jobs_iter = Mutex::new(jobs.into_iter());

    thread::scope(|scope| {
        for id in &cores {
            scope.spawn(|| {
                while let Some(job) = jobs_iter.lock().unwrap().next() {
                    let job_result =
                        run(runner, job, time_limit, mem_limit, Some(*id), show_stderr);
                    job_results.lock().unwrap().push(job_result);
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
    let child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    if !show_stderr {
        child.stderr(Stdio::null());
    }
    let mut child = child.spawn().unwrap();

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
