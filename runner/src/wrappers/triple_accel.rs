use super::*;

#[rustfmt::skip]
use ::triple_accel::*;

pub struct TripleAccel {
    costs: ::triple_accel::levenshtein::EditCosts,
    trace: bool,
}

impl AlignerParams for TripleAccelParams {
    type Aligner = TripleAccel;

    fn default(cost_model: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        // TripleAccel has a bug in traceback when using affine gap costs.
        assert!(!cost_model.is_affine());
        let costs = ::triple_accel::levenshtein::EditCosts::new(
            cost_model.sub as _,
            cost_model.extend as _,
            cost_model.open as _,
            None,
        );
        Self::Aligner { costs, trace }
    }
}

impl Aligner for TripleAccel {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let (cost, edits) =
            ::triple_accel::levenshtein::levenshtein_exp_with_opts(a, b, self.trace, self.costs);

        let cigar = edits.map(|edits| Cigar {
            operations: edits
                .into_iter()
                .map(|edit| {
                    (
                        match edit.edit {
                            EditType::Match => CigarOp::Match,
                            EditType::Mismatch => CigarOp::Sub,
                            EditType::AGap => CigarOp::Ins,
                            EditType::BGap => CigarOp::Del,
                            EditType::Transpose => unimplemented!(),
                        },
                        edit.count as _,
                    )
                })
                .collect(),
        });

        (cost as _, cigar)
    }
}
