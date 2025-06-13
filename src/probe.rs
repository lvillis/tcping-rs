//! Blocking TCP connect wrapped in `spawn_blocking` for stable RTT.
//!
//! Using `std::net::TcpStream::connect_timeout` avoids async/IOCP
//! scheduling jitter, while `spawn_blocking` keeps the async API.

use std::{
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

/// Perform one TCP connect with timeout on a blocking thread.
///
/// Returns `(success, rtt_ms)`.
pub async fn probe_once(addr: SocketAddr, to: Duration) -> (bool, f64) {
    tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let ok = TcpStream::connect_timeout(&addr, to).is_ok();
        let rtt = start.elapsed().as_secs_f64() * 1_000.0;
        (ok, rtt)
    })
        .await
        .unwrap_or((false, to.as_secs_f64() * 1_000.0))
}
