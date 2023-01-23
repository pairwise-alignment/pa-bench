use crate::{AlignStats, Stats};
use num::ToPrimitive;
use stats::Commute;

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

impl Commute for AlignStats {
    fn merge(&mut self, other: Self) {
        let AlignStats {
            files,
            seq_pairs,
            total_bases,
            length,
            divergence,
            largest_gap,
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

        self.substitutions += substitutions;
        self.insertions += insertions;
        self.deletions += deletions;
    }
}
