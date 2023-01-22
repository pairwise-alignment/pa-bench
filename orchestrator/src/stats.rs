use std::fs;
use std::path::Path;

use stats::*;

use itertools::Itertools;

use serde::Serialize;

use edlib_rs::*;

#[derive(Default)]
pub struct StatsCollector {
    len_minmax: MinMax<usize>,
    len_online: OnlineStats,
    divergence_minmax: MinMax<f64>,
    divergence_online: OnlineStats,
    largest_gap_minmax: MinMax<usize>,
    largest_gap_online: OnlineStats,
    substitutions: usize,
    insertions: usize,
    deletions: usize,
    total_bases: usize,
}

#[derive(Serialize)]
pub struct AlnStats {
    length: Stats<usize>,
    divergence: Stats<f64>,
    largest_gap: Stats<usize>,
    substitutions: usize,
    insertions: usize,
    deletions: usize,
    total_bases: usize,
}

#[derive(Serialize)]
pub struct Stats<T: Serialize> {
    min: T,
    max: T,
    mean: f64,
    stddev: f64,
}

impl StatsCollector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, file: &Path) {
        let data = fs::read(&file).expect("Could not read dataset file!");
        let mut config = EdlibAlignConfigRs::default();
        config.task = EdlibAlignTaskRs::EDLIB_TASK_PATH;
        data.split(|&c| c == b'\n')
            .tuples()
            .map(|(a, b)| {
                (
                    a.strip_prefix(b">").expect("Line must start with >"),
                    b.strip_prefix(b"<").expect("Line must start with <"),
                )
            })
            .for_each(|(a, b)| {
                let res = edlibAlignRs(&a, &b, &config);
                let aln = res.getAlignment().unwrap();
                let divergence = (res.getDistance() as f64) / (aln.len() as f64);
                let largest_gap = aln
                    .iter()
                    .group_by(|&o| o)
                    .into_iter()
                    .filter_map(|(&op, group)| {
                        if op == 1 || op == 2 {
                            // ins or del
                            Some(group.count())
                        } else {
                            None
                        }
                    })
                    .max()
                    .unwrap();

                aln.iter().for_each(|&op| match op {
                    1 => self.insertions += 1,
                    2 => self.deletions += 1,
                    3 => self.substitutions += 1,
                    _ => (),
                });

                self.len_minmax.add(a.len());
                self.len_minmax.add(b.len());
                self.len_online.add(a.len());
                self.len_online.add(b.len());
                self.divergence_minmax.add(divergence);
                self.divergence_online.add(divergence);
                self.largest_gap_minmax.add(largest_gap);
                self.largest_gap_online.add(largest_gap);
                self.total_bases += a.len() + b.len();
            });
    }

    pub fn finish(self) -> AlnStats {
        AlnStats {
            length: Stats {
                min: *self.len_minmax.min().unwrap(),
                max: *self.len_minmax.max().unwrap(),
                mean: self.len_online.mean(),
                stddev: self.len_online.stddev(),
            },
            divergence: Stats {
                min: *self.divergence_minmax.min().unwrap(),
                max: *self.divergence_minmax.max().unwrap(),
                mean: self.divergence_online.mean(),
                stddev: self.divergence_online.stddev(),
            },
            largest_gap: Stats {
                min: *self.largest_gap_minmax.min().unwrap(),
                max: *self.largest_gap_minmax.max().unwrap(),
                mean: self.largest_gap_online.mean(),
                stddev: self.largest_gap_online.stddev(),
            },
            substitutions: self.substitutions,
            insertions: self.insertions,
            deletions: self.deletions,
            total_bases: self.total_bases,
        }
    }
}
