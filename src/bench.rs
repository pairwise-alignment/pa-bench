use std::time::Instant;

use libc;

#[derive(Debug, Copy, Clone)]
pub struct Measured {
    runtime_secs: f64,
    memory_bytes: usize,
}

pub fn measure<F>(mut f: F, initial_mem: usize) -> Measured
where
    F: FnMut(),
{
    let start_time = Instant::now();

    f();

    Measured {
        runtime_secs: start_time.elapsed().as_secs_f64(),
        memory_bytes: get_maxrss().saturating_sub(initial_mem),
    }
}

pub fn get_maxrss() -> usize {
    let rusage = unsafe {
        let mut rusage = std::mem::MaybeUninit::uninit();
        libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr());
        rusage.assume_init()
    };
    let mut maxrss = rusage.ru_maxrss as usize;
    if cfg!(target_os = "macos") {
        // maxrss is in bytes
        maxrss *= 1;
    } else {
        // maxrss is in kilobytes
        maxrss *= 1024;
    }
    maxrss
}

pub fn set_limits(time_secs: usize, mem_kb: usize) {
    let set = |res, limit| {
        let rlimit = libc::rlimit {
            rlim_cur: limit as _,
            rlim_max: limit as _,
        };
        unsafe {
            libc::setrlimit(res, &rlimit);
        }
    };
    set(libc::RLIMIT_CPU, time_secs);
    set(libc::RLIMIT_AS, mem_kb);
}
