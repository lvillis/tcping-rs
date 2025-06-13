//! Pluggable output layer.

use crate::{
    cli::OutputMode,
    stats::{PingResult, Summary},
};
use serde_json::to_string;
use std::cell::Cell;

/// Print behaviour contract.
pub trait Formatter {
    fn probe(&self, res: &PingResult);
    fn summary(&self, sum: &Summary);
}

/* ---------- Normal text ---------- */

pub struct Normal;
impl Formatter for Normal {
    fn probe(&self, res: &PingResult) {
        let status = if res.success { "open" } else { "closed" };
        match res.jitter_ms {
            Some(j) => println!(
                "Probing {}/tcp - {status} - {:.4} ms jitter={:.4} ms",
                res.addr, res.duration_ms, j
            ),
            None => println!(
                "Probing {}/tcp - {status} - {:.4} ms",
                res.addr, res.duration_ms
            ),
        }
    }

    fn summary(&self, s: &Summary) {
        println!(
            "\n--- {} tcping statistics ---
{} probes sent, {} successful, {:.2}% packet loss",
            s.addr, s.total_attempts, s.successful_pings, s.packet_loss
        );
        if s.successful_pings > 0 {
            println!(
                "Round-trip min/avg/max = {:.4}/{:.4}/{:.4} ms",
                s.min_duration_ms, s.avg_duration_ms, s.max_duration_ms
            );
        }
    }
}

/* ---------- JSON ---------- */

pub struct Json;
impl Formatter for Json {
    fn probe(&self, res: &PingResult) {
        println!("{}", to_string(res).unwrap())
    }
    fn summary(&self, s: &Summary) {
        println!("{}", to_string(s).unwrap())
    }
}

/* ---------- CSV ---------- */

pub struct Csv;
impl Formatter for Csv {
    fn probe(&self, res: &PingResult) {
        let status = if res.success { "open" } else { "closed" };
        match res.jitter_ms {
            Some(j) => println!("{},{status},{:.4},{:.4}", res.addr, res.duration_ms, j),
            None => println!("{},{},{:.4}", res.addr, status, res.duration_ms),
        }
    }

    fn summary(&self, s: &Summary) {
        println!("address,total,success,loss,min,avg,max,resolve");
        println!(
            "{},{},{},{:.2},{:.4},{:.4},{:.4},{:.4}",
            s.addr,
            s.total_attempts,
            s.successful_pings,
            s.packet_loss,
            s.min_duration_ms,
            s.avg_duration_ms,
            s.max_duration_ms,
            s.resolve_time_ms
        );
    }
}

/* ---------- Markdown table ---------- */

pub struct Md {
    header_done: Cell<bool>,
}

impl Md {
    /// Construct a new Markdown formatter.
    pub fn new() -> Self {
        Self {
            header_done: Cell::new(false),
        }
    }
}

impl Default for Md {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter for Md {
    fn probe(&self, res: &PingResult) {
        // print header once
        if !self.header_done.replace(true) {
            println!("| address | status | rtt_ms | jitter_ms |");
            println!("|---------|--------|--------|-----------|");
        }

        let status = if res.success { "✓" } else { "✗" };
        let jitter = res
            .jitter_ms
            .map(|j| format!("{:.4}", j))
            .unwrap_or_else(|| "-".into());

        println!(
            "| {} | {} | {:.4} | {} |",
            res.addr, status, res.duration_ms, jitter
        );
    }

    fn summary(&self, s: &Summary) {
        println!("\n### Summary\n");
        println!("| field | value |");
        println!("|-------|-------|");
        println!("| address | {} |", s.addr);
        println!("| total probes | {} |", s.total_attempts);
        println!("| success | {} |", s.successful_pings);
        println!("| loss % | {:.2} |", s.packet_loss);
        println!(
            "| min / avg / max (ms) | {:.4} / {:.4} / {:.4} |",
            s.min_duration_ms, s.avg_duration_ms, s.max_duration_ms
        );
        println!("| resolve time (ms) | {:.4} |\n", s.resolve_time_ms);
    }
}

/* ---------- ANSI-colored TTY ---------- */

pub struct Color;
impl Formatter for Color {
    fn probe(&self, res: &PingResult) {
        let (status, color) = if res.success {
            ("open", "\x1b[32m") // green
        } else {
            ("closed", "\x1b[31m") // red
        };
        let reset = "\x1b[0m";
        match res.jitter_ms {
            Some(j) => println!(
                "Probing {}/tcp - {color}{status}{reset} - {:.4} ms jitter={:.4} ms",
                res.addr, res.duration_ms, j
            ),
            None => println!(
                "Probing {}/tcp - {color}{status}{reset} - {:.4} ms",
                res.addr, res.duration_ms
            ),
        }
    }

    fn summary(&self, s: &Summary) {
        let ok_color = "\x1b[32m";
        let bad_color = "\x1b[31m";
        let reset = "\x1b[0m";

        let color = if s.packet_loss == 0.0 {
            ok_color
        } else {
            bad_color
        };
        println!(
            "\n--- {} tcping statistics ---\n\
{} probes sent, {} successful, {color}{:.2}%{reset} packet loss",
            s.addr, s.total_attempts, s.successful_pings, s.packet_loss
        );
        if s.successful_pings > 0 {
            println!(
                "Round-trip min/avg/max = {:.4}/{:.4}/{:.4} ms",
                s.min_duration_ms, s.avg_duration_ms, s.max_duration_ms
            );
        }
    }
}

/* ---------- Factory ---------- */

pub fn from_mode(mode: OutputMode) -> Box<dyn Formatter> {
    match mode {
        OutputMode::Normal => Box::new(Normal),
        OutputMode::Json => Box::new(Json),
        OutputMode::Csv => Box::new(Csv),
        OutputMode::Md => Box::new(Md::new()),
        OutputMode::Color => Box::new(Color),
    }
}
