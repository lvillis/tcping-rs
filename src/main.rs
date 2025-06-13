//! Binary entry point for **tcping-rs**.
//!
//! * Parses CLI arguments.
//! * Runs the probing engine.
//! * On **Windows**:  
//!   - Requests a 1 ms system timer (`timeBeginPeriod`).  
//!   - Raises current thread priority to `THREAD_PRIORITY_HIGHEST`.

mod cli;
mod engine;
mod error;
mod formatter;
mod probe;
mod stats;

use clap::Parser;
use cli::Args;
use error::Result;

#[cfg(windows)]
mod win_boost {
    //! Lightweight FFI wrappers for high-resolution timing & priority.

    #[link(name = "winmm")]
    unsafe extern "system" {
        fn timeBeginPeriod(period: u32) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetThreadPriority(thread: *mut core::ffi::c_void, priority: i32) -> i32;
        fn GetCurrentThread() -> *mut core::ffi::c_void;
    }

    pub fn enable_high_res_timer() {
        // SAFETY: official API; 1 ms is always valid.
        unsafe { timeBeginPeriod(1) };
    }

    pub fn elevate_thread_priority() {
        const THREAD_PRIORITY_HIGHEST: i32 = 2;
        // SAFETY: current thread handle is always valid.
        unsafe {
            let th = GetCurrentThread();
            SetThreadPriority(th, THREAD_PRIORITY_HIGHEST);
        }
    }
}

fn main() -> Result<()> {
    #[cfg(windows)]
    {
        win_boost::enable_high_res_timer();
        win_boost::elevate_thread_priority();
    }

    let args = Args::parse();
    let exit_code = engine::run(args)?;
    std::process::exit(exit_code);
}
