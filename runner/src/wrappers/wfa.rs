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

    fn new(
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
            _ => unimplemented!("WFA does not support match bonus!"),
        };
        aligner.set_heuristic(self.heuristic);
        Ok(Self::Aligner { cm, aligner })
    }

    fn is_exact(&self) -> bool {
        self.heuristic == aligner::Heuristic::None
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
