mod bench;
use crate::bench::*;

use core_lib::*;

use std::io::{self, prelude::*};

use serde_json;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long, default_value_t = 3600usize)]
    time_limit_secs: usize,
    #[arg(short, long, default_value_t = 1048576usize)]
    mem_limit_kb: usize,
}

fn main() {
    let args = Args::parse();
    set_limits(args.time_limit_secs, args.mem_limit_kb);

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Error in reading from stdin!");
    let params: Parameters = serde_json::from_str(&input).expect("Error in parsing input json!");

    // TODO: read dataset

    // make sure initial memory usage is measured after reading in the dataset
    let initial_mem = get_maxrss();

    // TODO: run the correct algorithm based on params
}
