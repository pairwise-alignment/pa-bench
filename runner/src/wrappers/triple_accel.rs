use super::*;

#[rustfmt::skip]
use ::triple_accel::*;

pub struct TripleAccel {
    cost_model: CostModel,
    trace: bool,
}

impl AlignerParams for TripleAccelParams {
    type Aligner = TripleAccel;

    fn default(cost_model: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        TripleAccel { cost_model, trace }
    }
}

impl Aligner for TripleAccel {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let costs = ::triple_accel::levenshtein::EditCosts::new(
            self.cost_model.sub as _,
            self.cost_model.extend as _,
            self.cost_model.open as _,
            None,
        );
        let (cost, edits) =
            ::triple_accel::levenshtein::levenshtein_exp_with_opts(a, b, self.trace, costs);

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
