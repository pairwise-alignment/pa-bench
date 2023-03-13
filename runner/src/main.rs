mod bench;
mod wrappers;
use crate::{bench::*, wrappers::*};

use itertools::{izip, Itertools};
use pa_bench_types::*;
use pa_types::Seq;

use std::{
    cmp::max,
    fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
};

use serde_json;

use core_affinity;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about,
    after_help = "Input: json Job on stdin.
Output: json JobResult on stdout.
Exit code 101: Rust panic.
Exit code 102: aligner does not support the given parameters."
)]
struct Args {
    /// An optional experiment.yaml to run. By default takes a Job on stdin.
    experiment: Option<PathBuf>,

    /// Pin the process to the given core.
    #[arg(short, long)]
    pin_core_id: Option<usize>,

    /// Set the process niceness. <0 for higher priority.
    ///
    /// May require root rights, or modifying `/etc/security/limits.conf`.
    #[arg(long)]
    nice: Option<i32>,

    /// Disable time and memory limit.
    #[arg(long)]
    no_limits: bool,
}

fn read_path<'a>(
    path: &std::path::PathBuf,
    file_data: &'a mut Vec<u8>,
) -> Vec<(&'a [u8], &'a [u8])> {
    assert_eq!(
        path.extension()
            .expect("Dataset does not have a file extension"),
        "seq",
        "Job dataset {} does not have extension .seq.",
        path.display()
    );
    *file_data = fs::read(&path).expect("Could not read dataset file");
    file_data
        .split(|&c| c == '\n' as u8)
        .tuples()
        .map(|(a, b)| {
            (
                a.strip_prefix(b">").expect("Odd lines must start with >"),
                b.strip_prefix(b"<").expect("Even lines must start with <"),
            )
        })
        .collect()
}

fn main() {
    let args = Args::parse();
    if let Some(id) = args.pin_core_id {
        assert!(
            core_affinity::set_for_current(core_affinity::CoreId { id }),
            "Failed to pin to core!"
        );
    }
    if let Some(nice) = args.nice {
        rustix::process::nice(nice).unwrap();
    }

    if let Some(experiment) = &args.experiment {
        run_experiment(&args, experiment);
    } else {
        let mut stdin_job = vec![];
        io::stdin()
            .read_to_end(&mut stdin_job)
            .expect("Error in reading from stdin!");
        let job: Job = serde_json::from_slice(&stdin_job).expect("Error in parsing input json!");
        let output = run_job(&args, job);
        println!("{}", serde_json::to_string(&output).unwrap());
    }
}

fn run_job(args: &Args, job: Job) -> JobOutput {
    if !args.no_limits {
        set_limits(job.time_limit, job.mem_limit);
    }

    // NOTE: Although we could read and process the pairs in the dataset in streaming
    // manner, that complicates the time and memory measurement. Thus, all seqs are read up-front.

    // The seqs are references to the read file or direct input data.
    // This way all data is stored within one big allocation instead of being spread over many Vecs.
    let file_data = &mut vec![];
    let input_data;
    let sequence_pairs: Vec<(Seq, Seq)> = match &job.dataset {
        Dataset::Generated(generated_dataset) => read_path(&generated_dataset.path(), file_data),
        Dataset::File(path) => read_path(path, file_data),
        Dataset::Data(data) => {
            input_data = data.clone();
            input_data
                .iter()
                .map(|(a, b)| (a.as_bytes(), b.as_bytes()))
                .collect()
        }
    };

    let max_len = sequence_pairs
        .iter()
        .map(|(a, b)| max(a.len(), b.len()))
        .max()
        .unwrap_or(0);

    let mut costs = Vec::with_capacity(sequence_pairs.len());
    let mut cigars = Vec::with_capacity(sequence_pairs.len());
    let mut is_exact = false;

    let measured = measure(|| {
        let mut aligner;
        (aligner, is_exact) = get_aligner(&job.algo, job.costs, job.traceback, max_len);
        sequence_pairs.iter().for_each(|(a, b)| {
            let (cost, cigar) = aligner.align(a, b);
            costs.push(cost);
            if job.traceback {
                cigars.push(cigar.unwrap());
            }
        });
        aligner
    });

    // Verify the cigar strings, but do not return them as they are not used for further analysis and take a lot of space.
    if job.traceback {
        for ((a, b), &cost, cigar) in izip!(sequence_pairs, &costs, cigars) {
            let cigar_cost = cigar.verify(&job.costs, a, b);
            assert_eq!(
                cigar_cost,
                cost,
                "\nCIGAR COST IS NOT CORRECT\njob: {job:?}\nA: {}\nB: {}\nCigar: {:?}\ncost: {cost}\ncigar_cost: {cigar_cost}\n",
                String::from_utf8(a.to_vec()).unwrap(),
                String::from_utf8(b.to_vec()).unwrap(),
                cigar.to_string(),
            );
        }
    }

    let output = JobOutput {
        costs,
        exact_costs: None,
        //cigars,
        is_exact,
        p_correct: None,
        measured,
    };
    output
}

fn run_experiment(args: &Args, experiment: &Path) {
    let experiment_yaml = fs::read_to_string(&experiment).expect("Failed to read jobs generator:");
    let experiments: Experiments =
        serde_yaml::from_str(&experiment_yaml).expect("Failed to parse jobs generator yaml:");

    let jobs = experiments.generate(&Path::new("evals/data"), false, None, None);

    for (job, _stats) in jobs {
        let output = run_job(args, job);
        eprintln!("Job output:\n{output:?}");
    }
}
