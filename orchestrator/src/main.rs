use chrono::Timelike;
use clap::Parser;
use core_affinity;
use serde_json;
use serde_yaml;
use std::fs;
use std::io::prelude::*;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use wait4::{ResUse, Wait4};

use pa_bench_types::*;

#[derive(Debug, Parser)]
#[command(author, about)]
struct Args {
    // TODO: VEC
    /// Path to an experiment yaml file.
    #[arg(num_args = 1..)]
    experiments: Vec<PathBuf>,

    /// Path to the output json file. By default mirrors the `experiments` dir in `results`.
    ///
    /// Only works if only a single experiment is given.
    #[arg(short = 'o', long)]
    output: Option<PathBuf>,

    /// Shared cache of JobResults. Default: <experiment>.cache.json.
    #[arg(long)]
    cache: Option<PathBuf>,

    /// Number of parallel jobs to use.
    ///
    /// Jobs are pinned to separate cores.
    /// The number of jobs is capped to the total number of cores minus 1.
    #[arg(short = 'j', default_value = "5")]
    num_jobs: usize,

    /// Time limit. Defaults to value in experiment yaml or 1m.
    #[arg(short, long, value_parser = parse_duration::parse)]
    #[clap(help_heading = "Limits")]
    time_limit: Option<Duration>,

    /// Memory limit. Defaults to value in experiment yaml or 1GiB.
    #[arg(short, long, value_parser = parse_bytes)]
    #[clap(help_heading = "Limits")]
    mem_limit: Option<Bytes>,

    /// Process niceness. '--nice=-20' for highest priority.
    #[arg(long)]
    #[clap(help_heading = "Limits")]
    nice: Option<i32>,

    /// Ignore job cache, i.e. rerun jobs already present in the results file.
    ///
    /// By default, already-present jobs are reused.
    #[arg(long)]
    rerun_all: bool,

    /// Rerun failed jobs that are otherwise reused.
    ///
    /// This also reruns jobs that had at least as many resources. Useful when code changed.
    #[arg(long, conflicts_with = "rerun_all")]
    rerun_failed: bool,

    /// Regenerate generated datasets.
    #[arg(long, hide_short_help = true)]
    regenerate: bool,

    /// Discard all existing results.
    #[arg(long, hide_short_help = true)]
    clean: bool,

    /// Shorthand for '-j1 --nice=-20 --rerun_all'
    #[arg(long)]
    release: bool,

    /// Print jobs started and finished.
    #[arg(short, long)]
    #[clap(help_heading = "Output")]
    verbose: bool,

    /// Show stderr of runner process.
    #[arg(long)]
    #[clap(help_heading = "Output")]
    stderr: bool,

    /// Path to the data directory.
    #[arg(long, default_value = "evals/data")]
    #[clap(help_heading = "Custom paths")]
    #[clap(hide_short_help = true)]
    data_dir: PathBuf,

    /// Path to the runner binary. Uses $CARGO_MANIFEST_DIR/../target/release/runner by default.
    #[arg(long)]
    #[clap(help_heading = "Custom paths")]
    #[clap(hide_short_help = true)]
    runner: Option<PathBuf>,

    /// Path to the logs directory.
    ///
    /// Results of all runs are stored here.
    #[arg(long, default_value = "evals/results/.log")]
    #[clap(help_heading = "Custom paths")]
    #[clap(hide_short_help = true)]
    logs_dir: PathBuf,
}

fn main() {
    let mut args = Args::parse();

    // Handle `--release` flag.
    if args.release {
        args.nice.get_or_insert(-20);
        args.num_jobs = 1;
        args.rerun_all = true;
    }

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

    if args.output.is_some() && args.experiments.len() != 1 {
        panic!("Output can only be specified when running exactly 1 experiment.");
    }
    for i in 0..args.experiments.len() {
        run_experiment(&args, i);
    }
}

fn run_experiment(args: &Args, experiment_idx: usize) {
    let experiment = &args.experiments[experiment_idx];
    eprintln!("Running experiment {}", experiment.display());
    let experiment_yaml = fs::read_to_string(&experiment).expect("Failed to read jobs generator:");
    let experiments: Experiments =
        serde_yaml::from_str(&experiment_yaml).expect("Failed to parse jobs generator yaml:");

    let results_path = args.output.clone().unwrap_or_else(|| {
        // Mirror the structure of experiments in results.
        // To be precise: replace the deepest directory named "experiments" by "results".
        // If not found, simply uses .json instead of .yaml for the output file.
        let mut found = false;
        experiment
            .with_extension("json")
            .iter()
            .rev()
            .map(|c| {
                if c == "experiments" && !found {
                    found = true;
                    "results"
                } else {
                    c.to_str().unwrap()
                }
            })
            .rev()
            .collect()
    });
    let results_cache_path = if let Some(cache) = &args.cache {
        cache.clone()
    } else {
        results_path.with_extension("cache.json")
    };

    let mut jobs = experiments.generate(
        &args.data_dir,
        args.regenerate,
        args.time_limit,
        args.mem_limit,
    );
    eprintln!("Generated {} jobs.", jobs.len());

    // Read the cached results.
    let existing_jobs: Vec<JobResult> = if !args.clean && results_cache_path.is_file() {
        serde_json::from_str(
            &fs::read_to_string(&results_cache_path).expect("Error reading existing results file"),
        )
        .expect(&format!(
            "Error parsing results cache {}",
            results_cache_path.display()
        ))
    } else {
        vec![]
    };

    // We have a list of existing results, and a list of jobs to run.
    // We first split as follows:
    // - Existing jobs that are not part of the experiment.
    //   -> existing_jobs_extra
    // - Existing jobs that are reused.
    //   -> existing_jobs_used
    //
    // Then, if incremental running is set, we only retain jobs that are new.
    // - Existing jobs that are rerun.
    //   -> jobs_to_run
    // - New jobs.
    //   -> jobs_to_run
    //
    // Lastly, we remove from existing_jobs_used any job equal to a job_to_run.

    let (mut existing_jobs_used, existing_jobs_extra): (Vec<_>, Vec<_>) =
        existing_jobs.into_iter().partition(|existing_job| {
            jobs.iter()
                .find(|(j, _)| j.is_same_as(&existing_job.job))
                .is_some()
        });

    // Skip jobs that succeeded before, or were attempted with at least as many resources.
    if !args.rerun_all {
        eprintln!(
            "Cached jobs: {} in experiment + {} extra",
            existing_jobs_used.len(),
            existing_jobs_extra.len()
        );
        let num_jobs_before = jobs.len();
        jobs.retain(|(job, _stats)| {
            existing_jobs_used
                .iter()
                .find(|existing_job| {
                    existing_job.job.is_same_as(job)
                        && (existing_job.output.is_ok()
                            || (!args.rerun_failed
                                && existing_job.job.has_more_resources_than(job)))
                })
                .is_none()
        });
        eprintln!("Reused jobs: {}", num_jobs_before - jobs.len());
        eprintln!("Running {} jobs...", jobs.len());
    };

    let mut cores = core_affinity::get_core_ids()
        .unwrap()
        .into_iter()
        // NOTE: This assumes that virtual cores 0 and n/2 are on the same
        // physical core, in case hyperthreading is enabled.
        // TODO(ragnar): Is it better to spread the load over non-adjacent
        // physical cores? Unclear to me.
        .take(args.num_jobs + 1);

    // Reserve one core for the orchestrator.
    let orchestrator_core = cores.next().unwrap();
    core_affinity::set_for_current(orchestrator_core);

    // Remaining (up to) #processes cores are for runners.
    let runner_cores = cores.map(|c| c.id).collect();

    let job_results = run_with_threads(
        args.runner.as_ref().unwrap(),
        jobs,
        runner_cores,
        args.nice,
        args.stderr,
        args.verbose,
    );

    {
        let logs_path = args.logs_dir.join(format!(
            "{}_{}.json",
            experiment.file_stem().unwrap().to_str().unwrap(),
            chrono::Local::now()
                .with_nanosecond(0)
                .unwrap()
                .to_rfc3339()
        ));
        // Write results to persistent log.
        fs::create_dir_all(&args.logs_dir).unwrap();
        fs::write(&logs_path, &serde_json::to_string(&job_results).unwrap())
            .expect(&format!("Failed to write logs to {}", logs_path.display()));
    }

    if let Some(dir) = results_cache_path.parent() {
        fs::create_dir_all(dir).unwrap();
    }

    // Remove jobs that were run from existing results.
    existing_jobs_used.retain(|existing_job| {
        job_results
            .iter()
            .find(|job_result| job_result.job.is_same_as(&existing_job.job))
            .is_none()
    });

    let mut experiment_jobs = existing_jobs_used;
    experiment_jobs.extend(job_results);

    // First, write jobs for this experiment to '.exact.json'.
    eprintln!("Output: {}", results_path.display());
    fs::write(
        &results_path,
        &serde_json::to_string(&experiment_jobs).unwrap(),
    )
    .expect(&format!(
        "Failed to write results to {}",
        results_path.display()
    ));

    // Then, write the updated cache.
    let mut all_jobs = experiment_jobs;
    all_jobs.extend(existing_jobs_extra);

    eprintln!("Output: {}", results_cache_path.display());
    fs::write(
        &results_cache_path,
        &serde_json::to_string(&all_jobs).unwrap(),
    )
    .expect(&format!(
        "Failed to write results to {}",
        results_cache_path.display()
    ));

    verify_costs(&mut all_jobs);
}

/// Verify costs for exact algorithms and count correct costs for approximate algorithms.
/// Deduplicates exact costs.
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

        if output.costs.is_empty() {
            continue;
        }

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
            if reference_output.costs.is_empty() {
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
                // Remove the costs from this output, since they are the same as the reference output above.
                output.costs = vec![];
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
    jobs: Vec<(Job, DatasetStats)>,
    cores: Vec<usize>,
    nice: Option<i32>,
    show_stderr: bool,
    verbose: bool,
) -> Vec<JobResult> {
    let num_jobs = jobs.len();
    let job_results = Mutex::new(Vec::<JobResult>::with_capacity(jobs.len()));
    let jobs_iter = Mutex::new(jobs.into_iter());

    let running = Arc::new(Mutex::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || {
            eprintln!("Pressed Ctrl-C. Stopping running jobs.");
            *r.lock().unwrap() = false;
        })
        .expect("Error setting Ctrl-C handler");
    }

    #[derive(Default)]
    struct Counts {
        done: usize,
        success: usize,
        unsupported: usize,
        skipped: usize,
        failed: usize,
    }

    let counts = Mutex::new(Counts::default());
    let stderr = Mutex::new(());

    thread::scope(|scope| {
        for id in &cores {
            scope.spawn(|| {
                loop {
                    let Some((job, stats)) = jobs_iter.lock().unwrap().next() else {
                        break;
                    };
                    if !*running.lock().unwrap() {
                        break;
                    }
                    // If a smaller job for the same algorithm failed, skip it.
                    let mut skip = None;
                    if job.dataset.is_generated() {
                        for prev in job_results.lock().unwrap().iter() {
                            if prev.output.is_err()
                                && prev.job.dataset.is_generated()
                                && job.is_larger(&prev.job)
                            {
                                skip = Some(
                                    if prev.output.as_ref().unwrap_err() == &JobError::Unsupported {
                                        JobError::Unsupported
                                    } else {
                                        JobError::Skipped
                                    });
                                break;
                            }
                        }
                    }

                    let job_result = if let Some(err) = &skip {
                        JobResult {
                            job,
                            stats,
                            resources: ResourceUsage::default(),
                            output: Err(err.clone()),
                        }
                    } else {
                        if verbose {
                            eprintln!("\n Running job:\n{}\n", serde_json::to_string(&job).unwrap());
                        }
                        run_job(runner, job, stats, Some(*id), nice, show_stderr)
                    };

                    let _stderr = stderr.lock().unwrap();
                    if verbose {
                        eprintln!("\n Job result:\n{}\n Result: {:?}\n {:?}\n", serde_json::to_string(&job_result.job).unwrap(), job_result.output, job_result.resources);
                    }

                    let mut counts = counts.lock().unwrap();
                    counts.done += 1;
                    if job_result.output.is_ok() {
                        counts.success += 1;
                    } else if skip == Some(JobError::Skipped) {
                        counts.skipped += 1;
                    } else if *job_result.output.as_ref().unwrap_err() == JobError::Unsupported {
                        counts.unsupported += 1;
                    } else if *job_result.output.as_ref().unwrap_err() != JobError::Interrupted {
                        counts.failed += 1;
                        if !verbose {
                            eprintln!("\n Failed job:\n{}\n Result: {:?}\n {:?}\n", serde_json::to_string(&job_result.job).unwrap(), job_result.output, job_result.resources);
                        }
                    };
                    let Counts {
                        done,
                        success,
                        unsupported,
                        skipped,
                        failed,
                    } = *counts;
                    {
                        eprint!("\r Processed: {done:3} / {num_jobs:3}. Success {success:3}, Unsupported {unsupported:3}, Failed {failed:3}, Skipped {skipped}");
                    }

                    // If the orchestrator was aborted, do not push failing job results.
                    if job_result.output.is_ok() || *running.lock().unwrap() {
                        job_results.lock().unwrap().push(job_result);
                    }
                }
            });
        }
    });
    // Print a newline after the last count message
    eprintln!();

    job_results.into_inner().unwrap()
}

fn run_job(
    runner: &Path,
    job: Job,
    stats: DatasetStats,
    core_id: Option<usize>,
    nice: Option<i32>,
    show_stderr: bool,
) -> JobResult {
    let mut cmd = Command::new(runner);
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

    let start = Instant::now();
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    let ResUse { status, rusage } = child.wait4().unwrap();
    let walltime = start.elapsed().as_secs_f32();

    let resources = ResourceUsage {
        walltime,
        usertime: rusage.utime.as_secs_f32(),
        systemtime: rusage.stime.as_secs_f32(),
        maxrss: rusage.maxrss,
    };

    if status.success() {
        JobResult {
            job,
            stats,
            resources,
            output: Ok(serde_json::from_slice(&stdout).expect("Error reading output json:")),
        }
    } else {
        let err = if let Some(signal) = status.signal() {
            match signal {
                2 => JobError::Interrupted,
                6 => JobError::MemoryLimit,
                9 => JobError::Timeout,
                signal => JobError::Signal(signal),
            }
        } else if let Some(code) = status.code() {
            match code {
                101 => JobError::Panic,
                102 => JobError::Unsupported,
                code => JobError::ExitCode(code),
            }
        } else {
            panic!("Unknown exit type {:?}", status);
        };
        JobResult {
            job,
            stats,
            resources,
            output: Err(err),
        }
    }
}
