//! Aggregation logic & data models.

use crate::cli::Args;
use serde::Serialize;
use std::net::SocketAddr;

/// Per-probe result (JSON/CSV serialisable).
#[derive(Clone, Serialize)]
pub struct PingResult {
    pub success: bool,
    pub duration_ms: f64,
    pub jitter_ms: Option<f64>,
    pub addr: SocketAddr,
}

/// Final session summary.
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

/// Runtime statistics accumulator.
pub struct Stats {
    addr: SocketAddr,
    sent: usize,
    ok: usize,
    total_rtt: f64,
    min_rtt: f64,
    max_rtt: f64,
    resolve_ms: f64,
}

impl Stats {
    pub fn new(addr: SocketAddr, resolve_ms: f64) -> Self {
        Self {
            addr,
            sent: 0,
            ok: 0,
            total_rtt: 0.0,
            min_rtt: f64::MAX,
            max_rtt: 0.0,
            resolve_ms,
        }
    }

    /// Feed one probe result and construct `PingResult`.
    pub fn feed(&mut self, success: bool, rtt: f64, want_jitter: bool) -> PingResult {
        self.sent += 1;
        if success {
            self.ok += 1;
            self.total_rtt += rtt;
            self.min_rtt = self.min_rtt.min(rtt);
            self.max_rtt = self.max_rtt.max(rtt);
        }

        let jitter = if success && want_jitter && self.ok > 0 {
            Some((rtt - self.total_rtt / self.ok as f64).abs())
        } else {
            None
        };

        PingResult {
            success,
            duration_ms: rtt,
            jitter_ms: jitter,
            addr: self.addr,
        }
    }

    pub fn should_continue(&self, args: &Args) -> bool {
        args.continuous || self.sent < args.count
    }

    pub fn should_break(&self, success: bool, args: &Args) -> bool {
        success && args.exit_on_success
    }

    pub fn summary(&self) -> Summary {
        Summary {
            addr: self.addr,
            total_attempts: self.sent,
            successful_pings: self.ok,
            packet_loss: 100.0 * (1.0 - self.ok as f64 / self.sent as f64),
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

    pub fn exit_code(&self) -> i32 {
        if self.ok == self.sent && self.sent > 0 {
            0
        } else {
            1
        }
    }
}
