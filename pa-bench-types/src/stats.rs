use crate::{DatasetStats, Stats};
use itertools::Itertools;
use num::ToPrimitive;
use pa_types::CigarElem;
use pa_wrapper::wrappers::astarpa2::AstarPa2Params;
use stats::merge_all;
use stats::Commute;
use std::cmp::max;
use std::fs;
use std::path::Path;

impl<T: PartialOrd + Copy + ToPrimitive> Stats<T> {
    pub fn new(v: T) -> Self {
        Self {
            min: v,
            max: v,
            mean: v.to_f64().unwrap(),
            stddev: 0.0,
        }
    }
    pub fn merge(&mut self, other: Self, self_count: usize, other_count: usize) {
        self.min = if self.min < other.min {
            self.min
        } else {
            other.min
        };
        self.max = if self.max > other.max {
            self.max
        } else {
            other.max
        };

        let self_count = self_count as f64;
        let other_count = other_count as f64;

        // s1: sum of values
        let self_s1 = self_count * self.mean;
        let other_s1 = other_count * other.mean;

        // s2: sum of squares
        let self_s2 = ((self.stddev * self_count).powi(2) + self_s1 * self_s1) / self_count;
        let other_s2 = ((other.stddev * other_count).powi(2) + other_s1 * other_s1) / other_count;

        let cnt = self_count + other_count;
        let s1 = self_s1 + other_s1;
        let s2 = self_s2 + other_s2;

        // eprintln!(
        //     "Merge mean {} * {self_count} and {} * {other_count} into {} * {cnt}",
        //     self.mean,
        //     other.mean,
        //     s1 / cnt
        // );
        // eprintln!("Merge s2 {self_s2} and {other_s2} into {s2}",);
        self.mean = s1 / cnt;
        // Maximum with 0 in case stddev is 0.
        self.stddev = (cnt * s2 - s1 * s1).max(0.0).sqrt() / cnt;
    }
}

impl Commute for DatasetStats {
    fn merge(&mut self, other: Self) {
        let DatasetStats {
            files,
            seq_pairs,
            total_bases,
            length,
            divergence,
            largest_gap,
            edit_distance,
            substitutions,
            insertions,
            deletions,
        } = other;
        self.length.merge(length, 2 * self.seq_pairs, 2 * seq_pairs);
        self.divergence.merge(divergence, self.seq_pairs, seq_pairs);
        self.largest_gap
            .merge(largest_gap, self.seq_pairs, seq_pairs);

        self.files += files;
        self.seq_pairs += seq_pairs;
        self.total_bases += total_bases;

        self.edit_distance += edit_distance;
        self.substitutions += substitutions;
        self.insertions += insertions;
        self.deletions += deletions;
    }
}

pub fn file_stats(file: &Path) -> DatasetStats {
    eprintln!("Generating stats for {}", file.display());
    let data = fs::read(&file).expect("Could not read dataset file!");
    let mut aligner = AstarPa2Params::simple().make_aligner(true);
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
                let (cost, cigar) = aligner.align(&a, &b);
                let cigar = cigar.unwrap();

                // Compute the largest gap as the max over all intervals of the number of insertions (deletions) minus non-insertions (non-deletions).
                let mut largest_gap = 0;
                let mut ins = 0usize;
                let mut dels = 0usize;
                for CigarElem { op, cnt } in &cigar.ops {
                    let cnt = *cnt as usize;
                    match op {
                        pa_types::CigarOp::Ins => {
                            ins += cnt;
                            largest_gap = max(largest_gap, ins);

                            dels = dels.saturating_sub(cnt);
                        }
                        pa_types::CigarOp::Del => {
                            dels += cnt;
                            largest_gap = max(largest_gap, dels);

                            ins = ins.saturating_sub(cnt);
                        }
                        _ => {
                            ins = ins.saturating_sub(cnt);
                            dels = dels.saturating_sub(cnt);
                        }
                    }
                }

                let mut insertions = 0;
                let mut deletions = 0;
                let mut substitutions = 0;
                let mut cigar_len = 0;

                for CigarElem { op, cnt } in &cigar.ops {
                    let cnt = *cnt as usize;
                    cigar_len += cnt;
                    match op {
                        pa_types::CigarOp::Ins => insertions += cnt,
                        pa_types::CigarOp::Del => deletions += cnt,
                        pa_types::CigarOp::Sub => substitutions += cnt,
                        _ => (),
                    }
                }

                let divergence = (cost as f64) / (cigar_len as f64);

                let mut length = Stats::new(a.len());
                length.merge(Stats::new(b.len()), 1, 1);
                DatasetStats {
                    files: 0,
                    seq_pairs: 1,
                    total_bases: a.len() + b.len(),
                    length,
                    divergence: Stats::new(divergence),
                    largest_gap: Stats::new(largest_gap),
                    edit_distance: cost as _,
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
