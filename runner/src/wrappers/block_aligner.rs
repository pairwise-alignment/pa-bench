use super::*;

use pa_bench_types::BlockAlignerSize::*;

// Leading :: needs to be preserved to disambiguate the crate against this module.
#[rustfmt::skip]
use ::block_aligner::scan_block::*;
#[rustfmt::skip]
use ::block_aligner::scores::*;
#[rustfmt::skip]
use ::block_aligner::percent_len;

enum BlockAlignerBlock {
    Trace(Block<true, false>),
    NoTrace(Block<false, false>),
}

pub struct BlockAligner {
    params: BlockAlignerParams,
    matrix: NucMatrix,
    gaps: Gaps,
    block: BlockAlignerBlock,
    a: PaddedBytes,
    b: PaddedBytes,
    s: ScoreModel,
}

impl AlignerParams for BlockAlignerParams {
    type Aligner = BlockAligner;

    fn new(
        &self,
        cm: CostModel,
        trace: bool,
        max_len: usize,
    ) -> Result<Self::Aligner, &'static str> {
        if !cm.is_affine() {
            return Err("BlockAligner only works for affine cost models");
        }
        let max_size = match self.size {
            Size(_, max) => max,
            Percent(_, max) => percent_len(max_len, max),
        };
        let block = if trace {
            BlockAlignerBlock::Trace(Block::new(max_len, max_len, max_size))
        } else {
            BlockAlignerBlock::NoTrace(Block::new(max_len, max_len, max_size))
        };
        let s = ScoreModel::from_costs(cm);
        let matrix = NucMatrix::new_simple(s.r#match as i8, s.sub as i8);
        let gaps = Gaps {
            open: (s.open + s.extend) as i8,
            extend: s.extend as i8,
        };
        let a = PaddedBytes::new::<NucMatrix>(max_len, max_size);
        let b = PaddedBytes::new::<NucMatrix>(max_len, max_size);

        Ok(Self::Aligner {
            params: self.clone(),
            matrix,
            gaps,
            block,
            a,
            b,
            s,
        })
    }

    fn is_exact(&self) -> bool {
        false
    }
}

impl Aligner for BlockAligner {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>, AlignerStats) {
        let max_len = a.len().max(b.len());
        let size = match self.params.size {
            Size(min, max) => min..=max,
            Percent(min, max) => percent_len(max_len, min)..=percent_len(max_len, max),
        };
        self.a.set_bytes::<NucMatrix>(a, *size.end());
        self.b.set_bytes::<NucMatrix>(b, *size.end());
        match &mut self.block {
            BlockAlignerBlock::NoTrace(block) => {
                block.align(&self.a, &self.b, &self.matrix, self.gaps, size, 0);
                (
                    self.s.global_cost(block.res().score, a.len(), b.len()),
                    None,
                    AlignerStats::default(),
                )
            }
            BlockAlignerBlock::Trace(block) => {
                block.align(&self.a, &self.b, &self.matrix, self.gaps, size, 0);

                let mut ba_cigar = ::block_aligner::cigar::Cigar::new(self.a.len(), self.b.len());
                block
                    .trace()
                    .cigar_eq(&self.a, &self.b, self.a.len(), self.b.len(), &mut ba_cigar);
                let ops = (0..ba_cigar.len())
                    .map(|i| {
                        let ::block_aligner::cigar::OpLen { op, len } = ba_cigar.get(i);
                        let op = match op {
                            ::block_aligner::cigar::Operation::Eq => CigarOp::Match,
                            ::block_aligner::cigar::Operation::X => CigarOp::Sub,
                            // I and D are opposite of Ins and Del
                            ::block_aligner::cigar::Operation::D => CigarOp::Ins,
                            ::block_aligner::cigar::Operation::I => CigarOp::Del,
                            _ => unreachable!(),
                        };
                        CigarElem { op, cnt: len as _ }
                    })
                    .collect();

                (
                    self.s.global_cost(block.res().score, a.len(), b.len()),
                    Some(Cigar { ops }),
                    AlignerStats::default(),
                )
            }
        }
    }
}
