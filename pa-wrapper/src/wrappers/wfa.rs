use crate::*;
use rust_wfa2::{
    aligner::{
        AlignmentScope, AlignmentStatus, MemoryModel, WFAligner, WFAlignerEdit, WFAlignerGapAffine,
        WFAlignerGapLinear,
    },
    *,
};

fn default_memory_model() -> rust_wfa2::aligner::MemoryModel {
    rust_wfa2::aligner::MemoryModel::MemoryUltraLow
}
fn default_heuristic() -> rust_wfa2::aligner::Heuristic {
    rust_wfa2::aligner::Heuristic::None
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WfaParams {
    #[serde(default = "default_memory_model")]
    pub memory_model: rust_wfa2::aligner::MemoryModel,
    #[serde(default = "default_heuristic")]
    pub heuristic: rust_wfa2::aligner::Heuristic,
}

impl Default for WfaParams {
    fn default() -> Self {
        Self {
            memory_model: rust_wfa2::aligner::MemoryModel::MemoryUltraLow,
            heuristic: rust_wfa2::aligner::Heuristic::None,
        }
    }
}

pub struct Wfa {
    cm: CostModel,
    aligner: WFAligner,
}

impl AlignerParamsTrait for WfaParams {
    type Aligner = Wfa;

    fn build(
        &self,
        cm: CostModel,
        trace: bool,
        _max_len: usize,
    ) -> Result<Self::Aligner, &'static str> {
        // memory model does not matter if score only
        if !trace && self.memory_model != MemoryModel::MemoryUltraLow {
            return Err("WFA without trace should always use MmemoryModel::UltraLow");
        }
        let scope = if trace {
            AlignmentScope::Alignment
        } else {
            AlignmentScope::Score
        };
        let mut aligner = match cm {
            cm if cm.is_unit() => WFAlignerEdit::new(scope, self.memory_model),
            cm if cm.is_linear() => {
                WFAlignerGapLinear::new(cm.sub, cm.extend, scope, self.memory_model)
            }
            cm if cm.is_affine() => {
                WFAlignerGapAffine::new(cm.sub, cm.open, cm.extend, scope, self.memory_model)
            }
            _ => return Err("WFA does not support match bonus!"),
        };
        aligner.set_heuristic(self.heuristic);
        Ok(Self::Aligner { cm, aligner })
    }

    fn is_exact(&self) -> bool {
        self.heuristic == aligner::Heuristic::None
    }
}

impl AlignerTrait for Wfa {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let status = self.aligner.align_end_to_end(a, b);
        assert_eq!(status, AlignmentStatus::StatusSuccessful);
        let cost = self.aligner.score();
        let cigar = self.aligner.cigar();
        let cigar = if cigar.is_empty() {
            None
        } else {
            Some(Cigar::parse(&cigar, a, b))
        };
        let cost = if self.cm.is_unit() { cost } else { -cost };
        (cost as _, cigar, AlignerStats::default())
    }
}
