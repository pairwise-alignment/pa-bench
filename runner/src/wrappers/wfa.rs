use super::*;

use rust_wfa2::{
    aligner::{
        AlignmentScope, AlignmentStatus, MemoryModel, WFAligner, WFAlignerEdit, WFAlignerGapAffine,
        WFAlignerGapLinear,
    },
    *,
};

pub struct Wfa {
    cm: CostModel,
    aligner: WFAligner,
}

impl AlignerParams for WfaParams {
    type Aligner = Wfa;

    fn new(self, cm: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
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
            _ => unimplemented!("WFA does not support match bonus!"),
        };
        aligner.set_heuristic(self.heuristic);
        Wfa { cm, aligner }
    }

    fn default(cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner {
        Self {
            memory_model: MemoryModel::MemoryUltraLow,
            heuristic: aligner::Heuristic::None,
        }
        .new(cm, trace, max_len)
    }
}

impl Aligner for Wfa {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
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
        (cost as _, cigar)
    }
}
