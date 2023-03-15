#[cfg(feature = "parasailors")]
mod with_parasailors {
    use super::super::*;

    use parasailors::{global_alignment_score, Matrix, Profile};

    pub struct ParasailStriped {
        matrix: Matrix,
        gap_open: i32,
        gap_extend: i32,
        s: ScoreModel,
    }

    impl AlignerParams for ParasailStripedParams {
        type Aligner = ParasailStriped;

        fn new(
            &self,
            cm: CostModel,
            trace: bool,
            _max_len: usize,
        ) -> Result<Self::Aligner, &'static str> {
            if trace {
                return Err("Parasail does not support returning a trace");
            }
            let s = ScoreModel::from_costs(cm);
            Ok(Self::Aligner {
                matrix: Matrix::create("ACGT", s.r#match as _, s.sub as _),
                gap_open: -s.open - s.extend,
                gap_extend: -s.extend,
                s,
            })
        }

        fn is_exact(&self) -> bool {
            // FIXME: Turn this back to true after fixing overflows.
            false
        }
    }

    impl Aligner for ParasailStriped {
        fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
            let a_len = a.len();
            let a = Profile::new(a, &self.matrix);
            (
                self.s.global_cost(
                    global_alignment_score(&a, b, self.gap_open, self.gap_extend),
                    a_len,
                    b.len(),
                ),
                None,
            )
        }
    }
}

#[cfg(not(feature = "parasailors"))]
mod without_parasailors {
    use super::super::*;
    pub struct ParasailStriped;

    impl AlignerParams for ParasailStripedParams {
        type Aligner = ParasailStriped;

        fn new(
            &self,
            _cm: CostModel,
            _trace: bool,
            _max_len: usize,
        ) -> Result<Self::Aligner, &'static str> {
            return Err("Enable the parasailors feature to use Parasail");
        }

        fn is_exact(&self) -> bool {
            true
        }
    }

    impl Aligner for ParasailStriped {
        fn align(&mut self, _a: Seq, _b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
            unimplemented!("Enable parasailors feature to use Parasail.");
        }
    }
}
