use astar_pairwise_aligner::aligners::{
    astar::{AstarAligner, AstarPAParams},
    cigar::CigarElement,
};

use super::*;

pub struct AstarPA {
    trace: bool,
    aligner: Box<dyn AstarAligner>,
}

impl AlignerParams for AstarPAParams {
    type Aligner = AstarPA;

    fn new(&self, cm: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        assert!(cm.is_unit());
        AstarPA {
            trace,
            aligner: self.aligner(),
        }
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl Aligner for AstarPA {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        if self.trace {
            let (cost, cigar) = self.aligner.align(a, b);
            let cigar = Cigar {
                operations: cigar
                    .into_iter()
                    .filter_map(|&CigarElement { command, length }| match command {
                        astar_pairwise_aligner::aligners::cigar::CigarOp::Match => {
                            Some((CigarOp::Match, length as _))
                        }
                        astar_pairwise_aligner::aligners::cigar::CigarOp::Sub => {
                            Some((CigarOp::Sub, length as _))
                        }
                        astar_pairwise_aligner::aligners::cigar::CigarOp::Ins => {
                            Some((CigarOp::Ins, length as _))
                        }
                        astar_pairwise_aligner::aligners::cigar::CigarOp::Del => {
                            Some((CigarOp::Del, length as _))
                        }
                        _ => None,
                    })
                    .collect(),
            };
            (cost as _, Some(cigar))
        } else {
            (self.aligner.cost(a, b) as _, None)
        }
    }
}
