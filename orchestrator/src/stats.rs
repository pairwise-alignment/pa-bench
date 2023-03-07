use edlib_rs::*;
use itertools::Itertools;
use pa_bench_types::{AlignStats, Stats};
use stats::merge_all;
use std::cmp::max;
use std::fs;
use std::path::Path;

pub fn file_stats(file: &Path) -> AlignStats {
    eprintln!("Generating stats for {}", file.display());
    let data = fs::read(&file).expect("Could not read dataset file!");
    let mut config = EdlibAlignConfigRs::default();
    config.task = EdlibAlignTaskRs::EDLIB_TASK_PATH;
    let mut stats = merge_all(
        data.split(|&c| c == b'\n')
            .tuples()
            .map(|(a, b)| {
                (
                    a.strip_prefix(b">").expect("Line must start with >"),
                    b.strip_prefix(b"<").expect("Line must start with <"),
                )
            })
            .map(|(a, b)| {
                let res = edlibAlignRs(&a, &b, &config);
                let aln = res.getAlignment().unwrap();
                let divergence = (res.getDistance() as f64) / (aln.len() as f64);

                // Compute the largest gap as the max over all intervals of the number of insertions (deletions) minus non-insertions (non-deletions).
                let mut largest_gap = 0;
                let mut ins = 0usize;
                let mut dels = 0usize;
                for &op in aln {
                    match op {
                        1 => {
                            ins += 1;
                            largest_gap = max(largest_gap, ins);

                            dels = dels.saturating_sub(1);
                        }
                        2 => {
                            dels += 1;
                            largest_gap = max(largest_gap, dels);

                            ins = ins.saturating_sub(1);
                        }
                        _ => {
                            ins = ins.saturating_sub(1);
                            dels = dels.saturating_sub(1);
                        }
                    }
                }

                let mut insertions = 0;
                let mut deletions = 0;
                let mut substitutions = 0;

                aln.iter().for_each(|&op| match op {
                    1 => insertions += 1,
                    2 => deletions += 1,
                    3 => substitutions += 1,
                    _ => (),
                });
                let mut length = Stats::new(a.len());
                length.merge(Stats::new(b.len()), 1, 1);
                AlignStats {
                    files: 0,
                    seq_pairs: 1,
                    total_bases: a.len() + b.len(),
                    length,
                    divergence: Stats::new(divergence),
                    largest_gap: Stats::new(largest_gap),
                    edit_distance: res.getDistance() as _,
                    substitutions,
                    insertions,
                    deletions,
                }
            }),
    )
    .expect(&format!("File {} is empty!", file.display()));
    stats.files = 1;
    stats
}
