use crate::*;
use triple_accel::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TripleAccelParams;

pub struct TripleAccel {
    costs: ::triple_accel::levenshtein::EditCosts,
    trace: bool,
}

impl AlignerParamsTrait for TripleAccelParams {
    type Aligner = TripleAccel;

    fn build(
        &self,
        cm: CostModel,
        trace: bool,
        _max_len: usize,
    ) -> Result<Self::Aligner, &'static str> {
        if cm.is_affine() && trace {
            return Err("TripleAccel has a bug in traceback for affine costs");
        }
        let costs = ::triple_accel::levenshtein::EditCosts::new(
            cm.sub as _,
            cm.extend as _,
            cm.open as _,
            None,
        );
        Ok(Self::Aligner { costs, trace })
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl AlignerTrait for TripleAccel {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let (cost, edits) =
            ::triple_accel::levenshtein::levenshtein_exp_with_opts(a, b, self.trace, self.costs);

        let cigar = edits.map(|edits| Cigar {
            ops: edits
                .into_iter()
                .map(|edit| CigarElem {
                    op: match edit.edit {
                        EditType::Match => CigarOp::Match,
                        EditType::Mismatch => CigarOp::Sub,
                        EditType::AGap => CigarOp::Ins,
                        EditType::BGap => CigarOp::Del,
                        EditType::Transpose => unimplemented!(),
                    },
                    cnt: edit.count as _,
                })
                .collect(),
        });

        (cost as _, cigar, AlignerStats::default())
    }
}
