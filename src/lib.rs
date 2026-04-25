//! Library API for TCP reachability probes.
//!
//! The crate exposes a side-effect-free probing core. The `tcping` binary
//! builds CLI parsing, signal handling, and output formatting on top of this
//! API.
#![deny(unreachable_pub)]

mod error;
mod probe;
mod session;
mod stats;
mod target;
mod timestamp;

pub use error::{Result, TcpingError};
pub use session::{
    PingEvent, PingOptions, PingSession, ProbeCount, run_collect, run_collect_async,
    run_with_handler, run_with_handler_async, run_with_handler_until,
};
pub use stats::{OUTPUT_SCHEMA_V1, OUTPUT_SCHEMA_V2, PingResult, Summary, output_schema};
pub use target::{ResolvedTarget, Target, resolve_target};
pub use timestamp::RecordTimestamp;
