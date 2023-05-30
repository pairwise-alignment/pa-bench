use crate::*;
use edlib_rs::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EdlibParams;

pub struct Edlib {
    config: EdlibAlignConfigRs<'static>,
}

impl AlignerParamsTrait for EdlibParams {
    type Aligner = Edlib;

    fn build(
        &self,
        cm: CostModel,
        trace: bool,
        _max_len: usize,
    ) -> Result<Self::Aligner, &'static str> {
        if !cm.is_unit() {
            return Err("Edlib only works for unit cost model");
        }
        assert!(cm.is_unit());
        let mut config = EdlibAlignConfigRs::default();
        if trace {
            config.task = EdlibAlignTaskRs::EDLIB_TASK_PATH;
        }
        Ok(Self::Aligner { config })
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl AlignerTrait for Edlib {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let result = edlibAlignRs(a, b, &self.config);
        assert!(result.status == EDLIB_RS_STATUS_OK);
        let cost = result.getDistance();
        let cigar = result.getAlignment().map(|alignment| {
            Cigar::from_ops(alignment.into_iter().map(|op| match op {
                0 => CigarOp::Match,
                1 => CigarOp::Del,
                2 => CigarOp::Ins,
                3 => CigarOp::Sub,
                _ => panic!("Edlib should only return operations 0..=3."),
            }))
        });

        (cost, cigar, AlignerStats::default())
    }
}
