//! Takes a .seq file as input.
//! Outputs k=50 files each containing 1/50th of sequences, sorted by increasing divergence.

use clap::Parser;
use itertools::Itertools;
use pa_types::{CostModel, Seq, Sequence};
use pa_wrapper::{wrappers::astarpa2::AstarPa2Params, AlignerParams};
use rayon::prelude::*;
use std::{
    io::{BufWriter, Write},
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Parser, Debug)]
pub struct Args {
    #[clap(short, long, default_value_t = 50)]
    pub k: usize,

    pub input: PathBuf,
}

fn main() {
    let args = Args::parse();

    // File must be a .seq file.
    assert_eq!(args.input.extension().unwrap(), "seq");

    fn actg_only(s: Seq) -> Sequence {
        s.iter()
            .copied()
            .filter(|c| matches!(c, b'A' | b'C' | b'G' | b'T'))
            .collect_vec()
    }

    let input_file = std::fs::read(&args.input).unwrap();
    let pairs = input_file
        .split(|&c| c == b'\n')
        .tuples()
        .map(|(a, b)| (a.strip_prefix(b">").unwrap(), b.strip_prefix(b"<").unwrap()))
        .collect_vec();

    let num_pairs = pairs.len();
    eprintln!("Read {num_pairs} pairs");

    let done = AtomicUsize::new(0);

    let mut d_pairs = pairs
        .into_par_iter()
        .map(|(a, b)| {
            let mut aligner = AlignerParams::AstarPa2(AstarPa2Params::simple())
                .build_aligner(CostModel::unit(), false, 0)
                .0;
            let a = actg_only(a);
            let b = actg_only(b);
            let d = aligner.align(&a, &b).0 as f32 / (a.len() + b.len()) as f32;
            let done = done.fetch_add(1, Ordering::Relaxed);
            eprint!("{done:>3} / {num_pairs:>3}\r");
            (d, (a, b))
        })
        .collect::<Vec<_>>();
    d_pairs.sort_by(|(d1, _), (d2, _)| d1.partial_cmp(d2).unwrap());

    eprintln!();
    eprintln!("Sorted pairs");

    let chunk_size = d_pairs.len().div_ceil(args.k);

    let dir = args.input.with_extension("");
    std::fs::create_dir_all(&dir).unwrap();
    let digits = 1 + (args.k - 1).ilog(10) as usize;
    for (i, pairs) in d_pairs.chunks(chunk_size).enumerate() {
        let path = dir.join(format!("{:0digits$}.seq", i));
        let mut f = BufWriter::new(std::fs::File::create(&path).unwrap());
        eprintln!(
            "{}: {:>.4} -- {:>.4}",
            path.display(),
            pairs[0].0,
            pairs.last().unwrap().0
        );
        for (_d, (a, b)) in pairs {
            write!(f, ">").unwrap();
            f.write(a).unwrap();
            writeln!(f).unwrap();
            write!(f, "<").unwrap();
            f.write(b).unwrap();
            writeln!(f).unwrap();
        }
    }
}
