//! Low-level asynchronous TCP probe utilities.
//!
//! This module provides [`probe_once`], a helper that performs a single
//! non-blocking TCP `connect()` with a configurable timeout.  
//! It always records the real wall-clock round-trip time so callers can
//! distinguish “quickly refused” from “silently dropped”.

use std::net::SocketAddr;
use tokio::{
    net::TcpStream,
    time::{timeout, Duration, Instant},
};

/// Attempt one TCP connection and measure its round-trip time.
///
/// * `addr` – socket address to probe  
/// * `to`   – timeout for the connection attempt
///
/// Returns `(ok, rtt_ms)` where
/// * `ok` is `true` if the connection succeeded within `to`;  
/// * `rtt_ms` is the elapsed time in **milliseconds**.
pub async fn probe_once(addr: SocketAddr, to: Duration) -> (bool, f64) {
    let start = Instant::now();
    let res = timeout(to, TcpStream::connect(addr)).await;
    let rtt_ms = start.elapsed().as_secs_f64() * 1_000.0;

    match res {
        Ok(Ok(_stream)) => (true, rtt_ms),
        _ => (false, rtt_ms),
    }
}
