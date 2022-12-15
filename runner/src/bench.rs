use std::time::{Duration, Instant};

use libc;
use pa_bench_types::Measured;
use pa_types::Bytes;

pub fn measure<F>(f: F) -> Measured
where
    F: FnOnce(),
{
    let cpu_freq_start = get_cpu_freq();
    let cpu_clock_start = get_cpu_clock();
    let start_time = Instant::now();
    let initial_mem = get_maxrss();

    f();

    Measured {
        runtime: start_time.elapsed(),
        memory: get_maxrss().saturating_sub(initial_mem),
        cpu_freq_start,
        cpu_freq_end: get_cpu_freq(),
        cpu_clocks: get_cpu_clock().map(|c| c - cpu_clock_start.unwrap()),
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

fn get_cpu_freq() -> Option<f32> {
    // TODO(ragnar): check how accurate this returned value actually is.
    // TODO(ragnar): sanity check whether cur_cpu is the same as the pinned cpu.
    // NOTE: When the process is pinned to a single core this always returns the frequency of core 0.
    //let cur_cpu = unsafe { libc::sched_getcpu() };
    cpu_freq::get()[0 as usize].cur
}

fn get_cpu_clock() -> Option<u64> {
    if cfg!(any(target_arch = "x86_64")) {
        Some(unsafe { core::arch::x86_64::_rdtsc() })
    } else {
        None
    }
}
