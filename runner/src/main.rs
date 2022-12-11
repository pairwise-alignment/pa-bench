mod bench;
use crate::bench::*;

use itertools::Itertools;
use pa_bench_types::*;
use pa_types::*;

use std::{
    fs,
    io::{self, prelude::*},
    time::Duration,
};

use serde_json;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, value_parser = parse_duration::parse, default_value = "1h")]
    time_limit: Duration,
    #[arg(short, long, value_parser = parse_bytes, default_value = "1GiB")]
    mem_limit: Bytes,
}

fn main() {
    let args = Args::parse();
    set_limits(args.time_limit, args.mem_limit);

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

    // NOTE: Although we could read and process the pairs in the dataset in streaming manner, that complicates the time and memory measurement.
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
    let (measured, (costs, cigars)) = measure(|| {
        sequence_pairs
            .into_iter()
            .map(|(_a, _b)| -> (Cost, Cigar) {
                todo!("Align the sequences a and b using the algorithm and parameters of choice.");
            })
            .unzip()
    });
    let output = JobOutput {
        runtime: measured.runtime,
        memory: measured.memory,
        costs,
        cigars,
    };
    println!("{}", serde_json::to_string(&output).unwrap());
}
