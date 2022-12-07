use std::time::Instant;

pub struct Measured {
    runtime_secs: f64,
    memory_bytes: usize,
}

pub fn measure<F>(f: F) -> (f64, usize) where F: FnMut() {
    let initial_mem = get_maxrss();
    let start_time = Instant::now();

    f();

    Measured {
        runtime_secs: start_time.elapsed().as_secs_f64(),
        memory_bytes: get_maxrss().saturating_sub(initial_mem),
    }
}

fn get_maxrss() -> usize {
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
