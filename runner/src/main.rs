mod bench;
mod wrappers;
use crate::{bench::*, wrappers::*};

use itertools::Itertools;
use pa_bench_types::*;
use pa_types::*;

use std::{
    cmp::max,
    fs,
    io::{self, prelude::*},
    time::Duration,
};

use serde_json;

use core_affinity;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, value_parser = parse_duration::parse, default_value = "1h")]
    time_limit: Duration,
    #[arg(short, long, value_parser = parse_bytes, default_value = "1GiB")]
    mem_limit: Bytes,
    #[arg(short, long)]
    pin_core_id: Option<usize>,
    // process niceness. <0 for higher priority.
    #[arg(long)]
    nice: Option<i32>,
}

fn main() {
    let args = Args::parse();
    set_limits(args.time_limit, args.mem_limit);
    if let Some(id) = args.pin_core_id {
        assert!(
            core_affinity::set_for_current(core_affinity::CoreId { id }),
            "Failed to pin to core!"
        );
    }
    if let Some(nice) = args.nice {
        rustix::process::nice(nice).unwrap();
    }

    let mut stdin_job = vec![];
    io::stdin()
        .read_to_end(&mut stdin_job)
        .expect("Error in reading from stdin!");
    let job: Job = serde_json::from_slice(&stdin_job).expect("Error in parsing input json!");

    assert_eq!(
        job.dataset
            .extension()
            .expect("Dataset does not have a file extension"),
        "seq",
        "Job dataset {} does not have extension .seq.",
        job.dataset.display()
    );

    // NOTE: Although we could read and process the pairs in the dataset in streaming
    // manner, that complicates the time and memory measurement.
    let dataset = fs::read(&job.dataset).expect("Could not read dataset file");
    let sequence_pairs: Vec<(Seq, Seq)> = dataset
        .split(|&c| c == '\n' as u8)
        .tuples()
        .map(|(a, b)| {
            (
                a.strip_prefix(b">").expect("Odd lines must start with >"),
                b.strip_prefix(b"<").expect("Even lines must start with <"),
            )
        })
        .collect();
    let max_len = sequence_pairs
        .iter()
        .map(|(a, b)| max(a.len(), b.len()))
        .max()
        .unwrap_or(0);

    let mut costs = Vec::with_capacity(sequence_pairs.len());
    let mut cigars = Vec::with_capacity(sequence_pairs.len());

    let measured = measure(|| {
        let mut aligner = get_aligner(job.algo, job.costs, job.traceback, max_len);
        sequence_pairs.into_iter().for_each(|(a, b)| {
            let (cost, cigar) = aligner.align(&a, &b);
            costs.push(cost);
            if let Some(cigar) = cigar {
                cigars.push(cigar);
            }
        });
    });

    let output = JobOutput {
        costs,
        cigars,
        measured,
    };
    println!("{}", serde_json::to_string(&output).unwrap());
}
