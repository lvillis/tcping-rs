//! Runtime statistics and data structures.
//!
//! [Stats] accumulates per-probe results and emits a final [Summary].
//! Both [PingResult] and [Summary] are serde-serialisable so the
//! formatting layer can dump them directly.

use crate::cli::Args;
use serde::Serialize;
use std::net::SocketAddr;

pub const OUTPUT_SCHEMA: &str = "tcping.v1";

#[derive(Clone, Debug)]
struct P2Quantile {
    p: f64,
    initial: Vec<f64>,
    n: [i64; 5],
    np: [f64; 5],
    dn: [f64; 5],
    q: [f64; 5],
    count: i64,
}

impl P2Quantile {
    fn new(p: f64) -> Self {
        Self {
            p,
            initial: Vec::with_capacity(5),
            n: [0; 5],
            np: [0.0; 5],
            dn: [0.0; 5],
            q: [0.0; 5],
            count: 0,
        }
    }

    fn estimate_from_samples(samples: &[f64], p: f64) -> Option<f64> {
        if samples.is_empty() {
            return None;
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));

        let n = sorted.len();
        if n == 1 {
            return Some(sorted[0]);
        }

        let rank = p.clamp(0.0, 1.0) * (n as f64 - 1.0);
        let lo = rank.floor() as usize;
        let hi = rank.ceil() as usize;
        if lo == hi {
            Some(sorted[lo])
        } else {
            let t = rank - lo as f64;
            Some(sorted[lo] + (sorted[hi] - sorted[lo]) * t)
        }
    }

    fn init_from_first_five(&mut self) {
        self.initial.sort_by(|a, b| a.total_cmp(b));
        for (i, v) in self.initial.iter().copied().enumerate() {
            self.q[i] = v;
            self.n[i] = (i as i64) + 1;
        }

        self.np = [
            1.0,
            1.0 + 2.0 * self.p,
            1.0 + 4.0 * self.p,
            3.0 + 2.0 * self.p,
            5.0,
        ];
        self.dn = [0.0, self.p / 2.0, self.p, (1.0 + self.p) / 2.0, 1.0];
    }

    fn observe(&mut self, x: f64) {
        if self.count < 5 {
            self.initial.push(x);
            self.count += 1;
            if self.count == 5 {
                self.init_from_first_five();
            }
            return;
        }

        self.count += 1;

        let k = if x < self.q[0] {
            self.q[0] = x;
            0
        } else if x < self.q[1] {
            0
        } else if x < self.q[2] {
            1
        } else if x < self.q[3] {
            2
        } else if x <= self.q[4] {
            3
        } else {
            self.q[4] = x;
            3
        };

        for i in (k + 1)..5 {
            self.n[i] += 1;
        }
        for i in 0..5 {
            self.np[i] += self.dn[i];
        }

        for i in 1..=3 {
            let d = self.np[i] - self.n[i] as f64;
            if d >= 1.0 && (self.n[i + 1] - self.n[i]) > 1 {
                self.adjust_marker(i, 1);
            } else if d <= -1.0 && (self.n[i - 1] - self.n[i]) < -1 {
                self.adjust_marker(i, -1);
            }
        }
    }

    fn adjust_marker(&mut self, i: usize, sign: i64) {
        let sign_f = sign as f64;
        let n_im1 = self.n[i - 1] as f64;
        let n_i = self.n[i] as f64;
        let n_ip1 = self.n[i + 1] as f64;

        let q_im1 = self.q[i - 1];
        let q_i = self.q[i];
        let q_ip1 = self.q[i + 1];

        let q_par = q_i
            + sign_f / (n_ip1 - n_im1)
                * ((n_i - n_im1 + sign_f) * (q_ip1 - q_i) / (n_ip1 - n_i)
                    + (n_ip1 - n_i - sign_f) * (q_i - q_im1) / (n_i - n_im1));

        let q_new = if q_par > q_im1 && q_par < q_ip1 {
            q_par
        } else if sign > 0 {
            q_i + (q_ip1 - q_i) / (n_ip1 - n_i)
        } else {
            q_i - (q_i - q_im1) / (n_i - n_im1)
        };

        self.q[i] = q_new;
        self.n[i] += sign;
    }

    fn estimate(&self) -> Option<f64> {
        if self.count == 0 {
            return None;
        }

        if self.count <= 5 {
            return Self::estimate_from_samples(&self.initial, self.p);
        }

        Some(self.q[2])
    }
}

/// Result of a single probe.
///
/// This structure may be serialised as JSON / CSV by the formatter layer.
#[derive(Clone, Serialize)]
pub struct PingResult {
    pub schema: &'static str,
    pub record: &'static str,
    pub success: bool,
    pub duration_ms: f64,
    pub jitter_ms: Option<f64>,
    pub addr: SocketAddr,
}

/// Roll-up of an entire probing session.
#[derive(Serialize)]
pub struct Summary {
    pub schema: &'static str,
    pub record: &'static str,
    pub addr: SocketAddr,
    pub total_attempts: usize,
    pub successful_pings: usize,
    pub packet_loss: f64,
    pub min_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub resolve_time_ms: f64,
    pub jitter_p95_ms: Option<f64>,
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
    jitter_p95: Option<P2Quantile>,
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
            jitter_p95: None,
        }
    }

    /// Feed one probe result and obtain a [PingResult] to hand to the formatter.
    pub fn feed(&mut self, success: bool, rtt: f64, want_jitter: bool) -> PingResult {
        self.sent += 1;

        let jitter = if want_jitter && success {
            self.last_rtt.map(|prev| (rtt - prev).abs())
        } else {
            None
        };

        if let Some(j) = jitter {
            self.jitter_p95
                .get_or_insert_with(|| P2Quantile::new(0.95))
                .observe(j);
        }

        if success {
            self.ok += 1;
            self.total_rtt += rtt;
            self.min_rtt = self.min_rtt.min(rtt);
            self.max_rtt = self.max_rtt.max(rtt);
            self.last_rtt = Some(rtt);
        }

        PingResult {
            schema: OUTPUT_SCHEMA,
            record: "probe",
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
            schema: OUTPUT_SCHEMA,
            record: "summary",
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
            jitter_p95_ms: self.jitter_p95.as_ref().and_then(|q| q.estimate()),
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

    #[test]
    fn jitter_p95_is_reported_when_enabled() {
        let mut stats = Stats::new(loopback_addr(), 0.0);
        stats.feed(true, 10.0, true);
        stats.feed(true, 20.0, true); // jitter 10
        stats.feed(true, 25.0, true); // jitter 5
        stats.feed(true, 40.0, true); // jitter 15

        let summary = stats.summary();
        assert_eq!(summary.jitter_p95_ms, Some(14.5));
    }

    #[test]
    fn jitter_p95_is_none_when_disabled() {
        let mut stats = Stats::new(loopback_addr(), 0.0);
        stats.feed(true, 10.0, false);
        stats.feed(true, 20.0, false);

        let summary = stats.summary();
        assert_eq!(summary.jitter_p95_ms, None);
    }

    #[test]
    fn jitter_is_only_computed_for_successes() {
        let mut stats = Stats::new(loopback_addr(), 0.0);
        let first = stats.feed(true, 10.0, true);
        assert_eq!(first.jitter_ms, None);

        let failed = stats.feed(false, 10_000.0, true);
        assert_eq!(failed.jitter_ms, None);

        let second = stats.feed(true, 20.0, true);
        assert_eq!(second.jitter_ms, Some(10.0));
    }
}
