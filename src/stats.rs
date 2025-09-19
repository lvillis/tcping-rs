//! Runtime statistics and data structures.
//!
//! [Stats] accumulates per-probe results and emits a final [Summary].
//! Both [PingResult] and [Summary] are serde-serialisable so the
//! formatting layer can dump them directly.

use crate::cli::Args;
use serde::Serialize;
use std::net::SocketAddr;

/// Result of a single probe.
///
/// This structure may be serialised as JSON / CSV by the formatter layer.
#[derive(Clone, Serialize)]
pub struct PingResult {
    pub success: bool,
    pub duration_ms: f64,
    pub jitter_ms: Option<f64>,
    pub addr: SocketAddr,
}

/// Roll-up of an entire probing session.
#[derive(Serialize)]
pub struct Summary {
    pub addr: SocketAddr,
    pub total_attempts: usize,
    pub successful_pings: usize,
    pub packet_loss: f64,
    pub min_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub resolve_time_ms: f64,
}

/// Mutable accumulator used during a session.
pub struct Stats {
    addr: SocketAddr,
    sent: usize,
    ok: usize,
    total_rtt: f64,
    min_rtt: f64,
    max_rtt: f64,
    last_rtt: Option<f64>,
    resolve_ms: f64,
}

impl Stats {
    /// Create a new accumulator.
    pub fn new(addr: SocketAddr, resolve_ms: f64) -> Self {
        Self {
            addr,
            sent: 0,
            ok: 0,
            total_rtt: 0.0,
            min_rtt: f64::MAX,
            max_rtt: 0.0,
            last_rtt: None,
            resolve_ms,
        }
    }

    /// Feed one probe result and obtain a [PingResult] to hand to the formatter.
    pub fn feed(&mut self, success: bool, rtt: f64, want_jitter: bool) -> PingResult {
        self.sent += 1;

        let jitter = if want_jitter {
            self.last_rtt.map(|prev| (rtt - prev).abs())
        } else {
            None
        };

        if success {
            self.ok += 1;
            self.total_rtt += rtt;
            self.min_rtt = self.min_rtt.min(rtt);
            self.max_rtt = self.max_rtt.max(rtt);
            self.last_rtt = Some(rtt);
        }

        PingResult {
            success,
            duration_ms: rtt,
            jitter_ms: jitter,
            addr: self.addr,
        }
    }

    /// Should the main loop continue?
    pub fn should_continue(&self, args: &Args) -> bool {
        args.continuous || self.sent < args.count
    }

    /// Should we break early because -e/--exit-on-success?
    pub fn should_break(&self, success: bool, args: &Args) -> bool {
        success && args.exit_on_success
    }

    /// Produce the final [Summary].
    pub fn summary(&self) -> Summary {
        let packet_loss = if self.sent == 0 {
            0.0
        } else {
            100.0 * (1.0 - self.ok as f64 / self.sent as f64)
        };

        Summary {
            addr: self.addr,
            total_attempts: self.sent,
            successful_pings: self.ok,
            packet_loss,
            min_duration_ms: if self.ok > 0 { self.min_rtt } else { 0.0 },
            avg_duration_ms: if self.ok > 0 {
                self.total_rtt / self.ok as f64
            } else {
                0.0
            },
            max_duration_ms: if self.ok > 0 { self.max_rtt } else { 0.0 },
            resolve_time_ms: self.resolve_ms,
        }
    }

    /// Map statistics to a conventional Unix exit code.
    pub fn exit_code(&self) -> i32 {
        if self.ok == self.sent && self.sent > 0 {
            0
        } else {
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn loopback_addr() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80)
    }

    #[test]
    fn summary_handles_zero_probes() {
        let stats = Stats::new(loopback_addr(), 0.0);
        let summary = stats.summary();
        assert_eq!(summary.total_attempts, 0);
        assert_eq!(summary.packet_loss, 0.0);
    }

    #[test]
    fn jitter_is_difference_between_successive_successes() {
        let mut stats = Stats::new(loopback_addr(), 0.0);
        let first = stats.feed(true, 10.0, true);
        assert_eq!(first.jitter_ms, None);

        let second = stats.feed(true, 15.0, true);
        assert_eq!(second.jitter_ms, Some(5.0));
    }
}
