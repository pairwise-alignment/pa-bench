use std::{path::Path, time::Instant};

use chrono::SubsecRound;
use libc;
use pa_bench_types::{Bytes, Measured};

pub fn measure<F>(f: F) -> Measured
where
    F: FnOnce(),
{
    let cpu_start = get_cpu();
    let cpu_freq_start = cpu_start.and_then(|c| get_cpu_freq(c));
    let initial_mem = get_maxrss();
    let time_start = chrono::Utc::now().trunc_subsecs(3);
    let start = Instant::now();

    f();

    let runtime = start.elapsed().as_secs_f32();
    let time_end = chrono::Utc::now().trunc_subsecs(3);
    let memory = get_maxrss().saturating_sub(initial_mem);
    let cpu_end = get_cpu();
    let cpu_freq_end = cpu_end.and_then(|c| get_cpu_freq(c));
    Measured {
        // fill time-critical data first
        runtime,
        memory,
        time_start,
        time_end,
        cpu_start,
        cpu_end,
        cpu_freq_start,
        cpu_freq_end,
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

pub fn set_limits(time: u64, mem: Bytes) {
    let set = |res, limit| {
        let rlimit = libc::rlimit {
            rlim_cur: limit as _,
            rlim_max: limit as _,
        };
        unsafe {
            libc::setrlimit(res, &rlimit);
        }
    };
    set(libc::RLIMIT_CPU, time);
    set(libc::RLIMIT_DATA, mem);
}

fn get_cpu_freq(cur_cpu: i32) -> Option<f32> {
    let path = format!("/sys/devices/system/cpu/cpu{cur_cpu}/cpufreq/scaling_cur_freq");
    let path = &Path::new(&path);
    if !path.exists() {
        return None;
    }

    let val = std::fs::read_to_string(path).ok()?;
    Some(val.trim().parse::<f32>().ok()? / 1000.0)
}

fn get_cpu() -> Option<i32> {
    #[cfg(not(target_os = "macos"))]
    {
        Some(unsafe { libc::sched_getcpu() })
    }
    #[cfg(target_os = "macos")]
    {
        None
    }
}
