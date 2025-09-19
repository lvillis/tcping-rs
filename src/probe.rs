//! Blocking TCP connect wrapped in `spawn_blocking` for stable RTT.
//!
//! Using `std::net::TcpStream::connect_timeout` avoids async/IOCP
//! scheduling jitter, while `spawn_blocking` keeps the async API.

use std::{
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

fn clamp_to_timeout_ms(elapsed_ms: f64, timeout: Duration) -> f64 {
    let max_ms = timeout.as_secs_f64() * 1_000.0;
    if elapsed_ms > max_ms {
        max_ms
    } else {
        elapsed_ms
    }
}

/// Perform one TCP connect with timeout on a blocking thread.
///
/// Returns `(success, rtt_ms)`.
pub async fn probe_once(addr: SocketAddr, to: Duration) -> (bool, f64) {
    tokio::task::spawn_blocking(move || {
        let start = Instant::now();
        let ok = TcpStream::connect_timeout(&addr, to).is_ok();
        let elapsed_ms = start.elapsed().as_secs_f64() * 1_000.0;
        let rtt = clamp_to_timeout_ms(elapsed_ms, to);
        (ok, rtt)
    })
    .await
    .unwrap_or((false, to.as_secs_f64() * 1_000.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_limits_overshoot() {
        let timeout = Duration::from_millis(2000);
        assert_eq!(clamp_to_timeout_ms(1999.0, timeout), 1999.0);
        assert_eq!(clamp_to_timeout_ms(2005.0, timeout), 2000.0);
    }
}
