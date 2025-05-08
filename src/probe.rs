//! Low-level async TCP probe.

use tokio::{
    net::TcpStream,
    time::{timeout, Duration, Instant},
};
use std::net::SocketAddr;

/// Perform one TCP connect with timeout, return (success, rtt_ms).
pub async fn probe_once(addr: SocketAddr, to: Duration) -> (bool, f64) {
    let start = Instant::now();
    match timeout(to, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => (true, start.elapsed().as_secs_f64() * 1_000.0),
        _ => (false, to.as_secs_f64() * 1_000.0),
    }
}
