//! Binary entry point for **tcping-rs**.
//!
//! * Parses CLI arguments.
//! * Runs the probing engine.
//! * On **Windows**:
//!   - Requests a 1 ms system timer (`timeBeginPeriod`).
//!   - Raises current thread priority to `THREAD_PRIORITY_HIGHEST`.

use clap::Parser;
use std::process::ExitCode;
use tcping::{cli::Args, engine, error::Result};

#[cfg(windows)]
mod win_boost {
    //! Lightweight FFI wrappers for high-resolution timing & priority.

    #[link(name = "winmm")]
    unsafe extern "system" {
        fn timeBeginPeriod(period: u32) -> u32;
        fn timeEndPeriod(period: u32) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetThreadPriority(thread: *mut core::ffi::c_void, priority: i32) -> i32;
        fn GetCurrentThread() -> *mut core::ffi::c_void;
    }

    pub struct HighResTimerGuard {
        period: Option<u32>,
    }

    impl HighResTimerGuard {
        pub fn enable(period: u32) -> Self {
            let ok = unsafe { timeBeginPeriod(period) } == 0;
            Self {
                period: ok.then_some(period),
            }
        }
    }

    impl Drop for HighResTimerGuard {
        fn drop(&mut self) {
            if let Some(period) = self.period {
                unsafe { timeEndPeriod(period) };
            }
        }
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

fn main() -> Result<ExitCode> {
    #[cfg(windows)]
    let _timer_guard = win_boost::HighResTimerGuard::enable(1);

    #[cfg(windows)]
    win_boost::elevate_thread_priority();

    let args = Args::parse();
    let exit_code = engine::run(args)?;
    Ok(if exit_code == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
