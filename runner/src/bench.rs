use std::time::{Duration, Instant};

use libc;
use pa_types::Bytes;

#[derive(Debug, Copy, Clone)]
pub struct Measured {
    pub runtime: Duration,
    pub memory: Bytes,
}

pub fn measure<F>(f: F) -> Measured
where
    F: FnOnce(),
{
    let start_time = Instant::now();
    let initial_mem = get_maxrss();

    f();

    Measured {
        runtime: start_time.elapsed(),
        memory: get_maxrss().saturating_sub(initial_mem),
    }
}

/// Returns the maximum resident set size, ie the physical memory the program
/// uses, in bytes.
pub fn get_maxrss() -> Bytes {
    let rusage = unsafe {
        let mut rusage = std::mem::MaybeUninit::uninit();
        libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr());
        rusage.assume_init()
    };
    let maxrss = rusage.ru_maxrss as _;
    if cfg!(target_os = "macos") {
        // maxrss is in bytes
        maxrss
    } else {
        // maxrss is in kilobytes
        maxrss * 1024
    }
}

pub fn set_limits(time: Duration, mem: Bytes) {
    let set = |res, limit| {
        let rlimit = libc::rlimit {
            rlim_cur: limit as _,
            rlim_max: limit as _,
        };
        unsafe {
            libc::setrlimit(res, &rlimit);
        }
    };
    set(libc::RLIMIT_CPU, time.as_secs());
    set(libc::RLIMIT_AS, mem / 1024);
}
