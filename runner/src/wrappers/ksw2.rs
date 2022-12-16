use super::*;

use rust_wfa2::{
    aligner::{
        AlignmentScope, AlignmentStatus, MemoryModel, WFAligner, WFAlignerEdit, WFAlignerGapAffine,
        WFAlignerGapLinear,
    },
    *,
};

pub struct Wfa {
    aligner: WFAligner,
}

impl AlignerParams for WfaParams {
    type Aligner = Wfa;

    fn default(cost_model: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        let scope = if trace {
            AlignmentScope::Alignment
        } else {
            AlignmentScope::Score
        };
        let mut aligner = match cost_model {
            cm if cm.is_unit() => WFAlignerEdit::new(scope, MemoryModel::MemoryUltraLow),
            cm if cm.is_linear() => {
                WFAlignerGapLinear::new(cm.sub, cm.extend, scope, MemoryModel::MemoryUltraLow)
            }
            cm if cm.is_affine() => WFAlignerGapAffine::new(
                cm.sub,
                cm.open,
                cm.extend,
                scope,
                MemoryModel::MemoryUltraLow,
            ),
            _ => unimplemented!("WFA does not support match bonus!"),
        };
        aligner.set_heuristic(aligner::Heuristic::None);
        Wfa { aligner }
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
        (cost as _, cigar)
    }
}
