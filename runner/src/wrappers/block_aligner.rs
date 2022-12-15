use super::*;

// Leading :: needs to be preserved to disambiguate the crate against this module.
#[rustfmt::skip]
use ::block_aligner::scan_block::*;
#[rustfmt::skip]
use ::block_aligner::scores::*;

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
}

impl AlignerParams for BlockAlignerParams {
    type Aligner = BlockAligner;

    fn new(self, cm: CostModel, trace: bool, max_len: usize) -> Self::Aligner {
        let block = if trace {
            BlockAlignerBlock::Trace(Block::new(max_len, max_len, self.max_size))
        } else {
            BlockAlignerBlock::NoTrace(Block::new(max_len, max_len, self.max_size))
        };
        let CostModel {
            r#match,
            sub,
            open,
            extend,
        } = cm;
        let matrix = NucMatrix::new_simple(r#match as i8, -sub as i8);
        let gaps = Gaps {
            open: -(open + extend) as i8,
            extend: -extend as i8,
        };
        let a = PaddedBytes::new::<NucMatrix>(max_len, self.max_size);
        let b = PaddedBytes::new::<NucMatrix>(max_len, self.max_size);

        Self::Aligner {
            params: self,
            matrix,
            gaps,
            block,
            a,
            b,
        }
    }
}

impl Aligner for BlockAligner {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        self.a.set_bytes::<NucMatrix>(a, self.params.max_size);
        self.b.set_bytes::<NucMatrix>(b, self.params.max_size);
        match &mut self.block {
            BlockAlignerBlock::NoTrace(block) => {
                block.align(
                    &self.a,
                    &self.b,
                    &self.matrix,
                    self.gaps,
                    self.params.min_size..=self.params.max_size,
                    0,
                );
                (-block.res().score, None)
            }
            BlockAlignerBlock::Trace(block) => {
                block.align(
                    &self.a,
                    &self.b,
                    &self.matrix,
                    self.gaps,
                    self.params.min_size..=self.params.max_size,
                    0,
                );

                let mut ba_cigar = ::block_aligner::cigar::Cigar::new(self.a.len(), self.b.len());
                block
                    .trace()
                    .cigar(self.a.len(), self.b.len(), &mut ba_cigar);
                let (mut i, mut j) = (0, 0);
                let mut operations = Vec::with_capacity(ba_cigar.len());

                for idx in 0..ba_cigar.len() {
                    let ::block_aligner::cigar::OpLen { op, len } = ba_cigar.get(idx);

                    match op {
                        ::block_aligner::cigar::Operation::M => {
                            let mut prev = a[i] == b[j];
                            let mut curr_len = 1;
                            i += 1;
                            j += 1;

                            for _ in 1..len {
                                let eq = a[i] == b[j];
                                if prev == eq {
                                    curr_len += 1;
                                } else {
                                    operations.push((
                                        if prev { CigarOp::Match } else { CigarOp::Sub },
                                        curr_len,
                                    ));
                                    prev = eq;
                                    curr_len = 1;
                                }
                                i += 1;
                                j += 1;
                            }

                            operations
                                .push((if prev { CigarOp::Match } else { CigarOp::Sub }, curr_len));
                        }
                        ::block_aligner::cigar::Operation::I => {
                            i += len;
                            operations.push((CigarOp::Ins, len as _));
                        }
                        ::block_aligner::cigar::Operation::D => {
                            j += len;
                            operations.push((CigarOp::Del, len as _));
                        }
                        _ => (),
                    }
                }

                (-block.res().score, Some(Cigar { operations }))
            }
        }
    }
}
