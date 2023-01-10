use std::{
    path::Path,
    time::{Duration, Instant},
};

use chrono::Timelike;
use libc;
use pa_bench_types::Measured;
use pa_types::Bytes;

pub fn measure<F>(f: F) -> Measured
where
    F: FnOnce(),
{
    let time_start = chrono::Utc::now().with_nanosecond(0).unwrap();
    let cpu_freq_start = get_cpu_freq();
    let start_time = Instant::now();
    let initial_mem = get_maxrss();

    f();

    Measured {
        // fill time-critical data first
        runtime: start_time.elapsed().as_secs_f32(),
        time_end: chrono::Utc::now().with_nanosecond(0).unwrap(),
        memory: get_maxrss().saturating_sub(initial_mem),
        cpu_freq_end: get_cpu_freq(),
        time_start,
        cpu_freq_start,
    }
}

/// Returns the maximum resident set size, i.e. the physical memory the program
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
    set(libc::RLIMIT_DATA, mem);
}

fn get_cpu_freq() -> Option<f32> {
    let cur_cpu = unsafe { libc::sched_getcpu() };
    let path = format!("/sys/devices/system/cpu/cpu{cur_cpu}/cpufreq/scaling_cur_freq");
    let path = &Path::new(&path);
    if !path.exists() {
        return None;
    }

    let val = std::fs::read_to_string(path).ok()?;
    Some(val.trim().parse::<f32>().ok()? / 1000.0)
}
