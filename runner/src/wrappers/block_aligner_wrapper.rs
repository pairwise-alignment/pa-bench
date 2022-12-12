use super::*;

use block_aligner::scan_block::*;
use block_aligner::scores::*;

pub struct BlockAlignerWrapper {
    params: BlockAlignerParams,
    matrix: NucMatrix,
    gaps: Gaps,
    block: BlockAligner,
    a: PaddedBytes,
    b: PaddedBytes,
}

enum BlockAligner {
    Trace(Block<true, false>),
    NoTrace(Block<false, false>),
}

impl BlockAlignerWrapper {
    pub fn new(
        max_len: usize,
        params: BlockAlignerParams,
        costs: CostModel,
        traceback: bool,
    ) -> Self {
        let block = if traceback {
            BlockAligner::Trace(Block::new(max_len, max_len, params.max_size))
        } else {
            BlockAligner::NoTrace(Block::new(max_len, max_len, params.max_size))
        };
        let (matrix, gaps) = if let CostModel::Affine {
            r#match,
            sub,
            open,
            extend,
        } = costs
        {
            (
                NucMatrix::new_simple(r#match as i8, -sub as i8),
                Gaps {
                    open: -(open + extend) as i8,
                    extend: -extend as i8,
                },
            )
        } else {
            unimplemented!()
        };
        let a = PaddedBytes::new::<NucMatrix>(max_len, params.max_size);
        let b = PaddedBytes::new::<NucMatrix>(max_len, params.max_size);

        Self {
            params,
            matrix,
            gaps,
            block,
            a,
            b,
        }
    }
}

impl Wrapper for BlockAlignerWrapper {
    fn cost(&mut self, a: Seq, b: Seq) -> Cost {
        self.a.set_bytes::<NucMatrix>(a, self.params.max_size);
        self.b.set_bytes::<NucMatrix>(b, self.params.max_size);
        if let BlockAligner::NoTrace(block) = &mut self.block {
            block.align(
                &self.a,
                &self.b,
                &self.matrix,
                self.gaps,
                self.params.min_size..=self.params.max_size,
                0,
            );
            block.res().score
        } else {
            unimplemented!()
        }
    }

    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Cigar) {
        todo!()
    }
}
