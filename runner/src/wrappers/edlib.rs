use super::*;

use edlib_rs::edlibrs::*;

pub struct Edlib {
    trace: bool,
}

impl AlignerParams for EdlibParams {
    type Aligner = Edlib;

    fn new(self, cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner {
        Self::default(cm, trace, max_len)
    }

    fn default(cm: CostModel, trace: bool, _max_len: usize) -> Self::Aligner {
        assert!(cm.is_unit());
        Edlib { trace }
    }
}

impl Aligner for Edlib {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let mut config = EdlibAlignConfigRs::default();
        if self.trace {
            config.task = EdlibAlignTaskRs::EDLIB_TASK_PATH;
        }
        let result = edlibAlignRs(a, b, &config);
        assert!(result.status == EDLIB_STATUS_OK);
        let score = result.getDistance();
        let cigar = result.getAlignment().map(|alignment| {
            Cigar::from_ops(alignment.into_iter().map(|op| match op {
                0 => CigarOp::Match,
                1 => CigarOp::Del,
                2 => CigarOp::Ins,
                3 => CigarOp::Sub,
                _ => panic!("Edlib should only return operations 0..=3."),
            }))
        });
        (score, cigar)
    }
}
