use super::*;

use ksw2_sys::*;
use libc::c_void;

const M: usize = 4 + 1;
pub struct Ksw2 {
    params: Ksw2Params,
    trace: bool,
    encoding: [u8; 256],
    // NOTE: This matrix must be padded with a row and column of 0 at the end for ksw2_extz_sse to work.
    score_matrix: [i8; M * M],
    open: i8,
    extend: i8,
}

impl AlignerParams for Ksw2Params {
    type Aligner = Ksw2;
    fn new(
        &self,
        cm: CostModel,
        trace: bool,
        _max_len: usize,
    ) -> Result<Self::Aligner, &'static str> {
        // NOTE: The score matrix isn't actually used: only m[0]=match and
        // m[1]=mismatch are used, unless the flag KSW_EZ_GENERIC_SC=0x04 is set which says the matrix is arbitrary.
        let mut score_matrix = [0; M * M];
        for i in 0..M - 1 {
            for j in 0..M - 1 {
                score_matrix[(M * i + j) as usize] = if i == j { 0 } else { -cm.sub as i8 };
            }
        }

        let mut encoding: [u8; 256] = [0; 256];
        encoding[b'A' as usize] = 0;
        encoding[b'C' as usize] = 1;
        encoding[b'G' as usize] = 2;
        encoding[b'T' as usize] = 3;
        Ok(Self::Aligner {
            params: self.clone(),
            trace,
            encoding,
            score_matrix,
            open: cm.open as _,
            extend: cm.extend as _,
        })
    }

    fn is_exact(&self) -> bool {
        true
    }
}

impl Aligner for Ksw2 {
    fn align(&mut self, a: Seq, b: Seq) -> (Cost, Option<Cigar>) {
        let a_mapped: Vec<u8> = a.iter().map(|x| self.encoding[*x as usize]).collect();
        let b_mapped: Vec<u8> = b.iter().map(|x| self.encoding[*x as usize]).collect();
        unsafe {
            let score;
            // Returned length of cigar. Out-only.
            let mut n_cigar: i32 = 0;
            // Allocated cigar. Must be free afterwards.
            let mut ksw2_cigar: *mut u32 = std::ptr::null_mut();

            // Documentation is at
            // https://github.com/lh3/ksw2/blob/06b2183b0f6646d82f2e3f5884008a1b4582f5b5/ksw2.h#L44.
            match self.params.method {
                Ksw2Method::GlobalGreen
                | Ksw2Method::GlobalSuzuki
                | Ksw2Method::GlobalSuzukiSse => {
                    // Length of allocated cigar memory. In-out.
                    let mut m_cigar: i32 = 0;

                    if self.trace {
                        ksw2_cigar = libc::malloc(16) as *mut u32;
                        m_cigar = 4;
                    }

                    let function = match (self.params.band_doubling, self.params.method) {
                        (false, Ksw2Method::GlobalGreen) => ksw_gg,
                        (false, Ksw2Method::GlobalSuzuki) => ksw_gg2,
                        (false, Ksw2Method::GlobalSuzukiSse) => ksw_gg2_sse,
                        (true, Ksw2Method::GlobalGreen) => ksw_gg_band_doubling,
                        (true, Ksw2Method::GlobalSuzuki) => ksw_gg2_band_doubling,
                        (true, Ksw2Method::GlobalSuzukiSse) => ksw_gg2_sse_band_doubling,
                        _ => unreachable!(),
                    };

                    score = function(
                        // don't use a kmalloc memory pool.
                        std::ptr::null_mut(),
                        // Input sequences
                        a_mapped.len() as i32,
                        a_mapped.as_ptr(),
                        b_mapped.len() as i32,
                        b_mapped.as_ptr(),
                        M as i8,
                        // Scoring matrix and gap penalties
                        self.score_matrix.as_ptr(),
                        self.open,
                        self.extend,
                        // band: disabled
                        -1,
                        (&mut m_cigar) as *mut i32,
                        (&mut n_cigar) as *mut i32,
                        (&mut ksw2_cigar) as *mut *mut u32,
                    );
                }

                Ksw2Method::ExtensionGreen => {
                    let mut output: ksw_extz_t = std::mem::zeroed();
                    let function = if self.params.band_doubling {
                        ksw_extz_band_doubling
                    } else {
                        ksw_extz
                    };
                    function(
                        // don't use a kmalloc memory pool.
                        std::ptr::null_mut(),
                        // Input sequences
                        a_mapped.len() as i32,
                        a_mapped.as_ptr(),
                        b_mapped.len() as i32,
                        b_mapped.as_ptr(),
                        M as i8,
                        // Scoring matrix and gap penalties
                        self.score_matrix.as_ptr(),
                        self.open,
                        self.extend,
                        // band: disabled
                        -1,
                        // zdrop: disabled
                        -1,
                        // TODO(ragnar): Figure out what this means.
                        // end_bonus, needed for ksw_extz2_sse
                        // 0
                        // flag:
                        // https://github.com/lh3/ksw2/blob/master/ksw2.h#L8
                        // (seems like it can just be 0 for our use case)
                        if self.trace { 0 } else { 1 },
                        &mut output,
                    );
                    score = output.score;
                    n_cigar = output.n_cigar;
                    ksw2_cigar = output.cigar;
                }
                Ksw2Method::ExtensionSuzukiSse => {
                    let mut output: ksw_extz_t = std::mem::zeroed();
                    let function = if self.params.band_doubling {
                        ksw_extz2_sse_band_doubling
                    } else {
                        ksw_extz2_sse
                    };
                    function(
                        // don't use a kmalloc memory pool.
                        std::ptr::null_mut(),
                        // Input sequences
                        a_mapped.len() as i32,
                        a_mapped.as_ptr(),
                        b_mapped.len() as i32,
                        b_mapped.as_ptr(),
                        M as i8,
                        // Scoring matrix and gap penalties
                        self.score_matrix.as_ptr(),
                        self.open,
                        self.extend,
                        // band: disabled
                        -1,
                        // zdrop: disabled
                        -1,
                        // TODO(ragnar): Figure out what this means.
                        // end_bonus, needed for ksw_extz2_sse
                        // Shouldn't matter since the extension-only flag KSW_EZ_EXTZ_ONLY is not set.
                        0,
                        // flag:
                        // https://github.com/lh3/ksw2/blob/master/ksw2.h#L8
                        // (seems like it can just be 0 for our use case)
                        if self.trace { 0 } else { 1 },
                        &mut output,
                    );
                    score = output.score;
                    n_cigar = output.n_cigar;
                    ksw2_cigar = output.cigar;
                }
                Ksw2Method::DualAffineExtensionGreen => todo!(),
                Ksw2Method::DualAffineExtensionSuzukiSse => todo!(),
            };
            let cigar = self.trace.then(|| {
                let cigar = std::slice::from_raw_parts_mut(ksw2_cigar, n_cigar as usize);
                let cigar = Cigar::resolve_matches(
                    cigar.into_iter().map(|&mut val| {
                        CigarElem {
                            op: match val & 15 {
                                // NOTE: This Match will be resolved to Match or Sub as needed.
                                0 => CigarOp::Match,
                                1 => CigarOp::Del,
                                2 => CigarOp::Ins,
                                7 => CigarOp::Match,
                                8 => CigarOp::Sub,
                                _ => panic!(),
                            },
                            cnt: val as I / 16,
                        }
                    }),
                    &a_mapped,
                    &b_mapped,
                );
                libc::free(ksw2_cigar as *mut c_void);
                cigar
            });
            let cost = -score;
            (cost, cigar)
        }
    }
}
