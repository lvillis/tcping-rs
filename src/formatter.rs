//! Pluggable output layer.

use crate::{
    cli::{OutputMode, TimestampFormat},
    stats::{PingResult, Summary},
    timestamp::RecordTimestamp,
};
use serde::Serialize;
use serde_json::to_string;
use std::cell::Cell;

/// Print behaviour contract.
pub trait Formatter {
    fn probe(&self, res: &PingResult);
    fn summary(&self, sum: &Summary);
}

/* ---------- Normal text ---------- */

fn human_timestamp(timestamp: Option<&RecordTimestamp>, format: Option<TimestampFormat>) -> String {
    match (timestamp, format) {
        (Some(timestamp), Some(format)) => format!("[{}] ", timestamp.render(format)),
        _ => String::new(),
    }
}

pub struct Normal {
    timestamp_format: Option<TimestampFormat>,
}

impl Normal {
    pub fn new(timestamp_format: Option<TimestampFormat>) -> Self {
        Self { timestamp_format }
    }

    fn render_probe(&self, res: &PingResult) -> String {
        let prefix = human_timestamp(res.timestamp.as_ref(), self.timestamp_format);
        let status = if res.success { "open" } else { "closed" };
        match res.jitter_ms {
            Some(j) => format!(
                "{prefix}Probing {}/tcp - {status} - {:.4} ms jitter={:.4} ms",
                res.addr, res.duration_ms, j
            ),
            None => format!(
                "{prefix}Probing {}/tcp - {status} - {:.4} ms",
                res.addr, res.duration_ms
            ),
        }
    }
}

impl Formatter for Normal {
    fn probe(&self, res: &PingResult) {
        println!("{}", self.render_probe(res));
    }

    fn summary(&self, s: &Summary) {
        let prefix = human_timestamp(s.timestamp.as_ref(), self.timestamp_format);
        println!(
            "\n{prefix}--- {} tcping statistics ---
{} probes sent, {} successful, {:.2}% packet loss",
            s.addr, s.total_attempts, s.successful_pings, s.packet_loss
        );
        if s.successful_pings > 0 {
            println!(
                "Round-trip min/avg/max = {:.4}/{:.4}/{:.4} ms",
                s.min_duration_ms, s.avg_duration_ms, s.max_duration_ms
            );
        }
        if s.resolve_time_ms > 0.0 {
            println!("Address resolved in {:.4} ms", s.resolve_time_ms);
        }
        if let Some(j95) = s.jitter_p95_ms {
            println!("Jitter p95 = {:.4} ms", j95);
        }
    }
}

/* ---------- JSON ---------- */

fn round_to(value: f64, decimals: i32) -> f64 {
    let factor = 10_f64.powi(decimals);
    (value * factor).round() / factor
}

fn round2(value: f64) -> f64 {
    round_to(value, 2)
}

fn round4(value: f64) -> f64 {
    round_to(value, 4)
}

#[derive(Serialize)]
struct JsonProbe {
    schema: &'static str,
    record: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp_unix_ms: Option<i64>,
    success: bool,
    duration_ms: f64,
    jitter_ms: Option<f64>,
    addr: std::net::SocketAddr,
}

impl From<&PingResult> for JsonProbe {
    fn from(res: &PingResult) -> Self {
        Self {
            schema: res.schema,
            record: res.record,
            timestamp: res.timestamp.as_ref().map(|ts| ts.rfc3339().to_string()),
            timestamp_unix_ms: res.timestamp.as_ref().map(RecordTimestamp::unix_ms),
            success: res.success,
            duration_ms: round4(res.duration_ms),
            jitter_ms: res.jitter_ms.map(round4),
            addr: res.addr,
        }
    }
}

#[derive(Serialize)]
struct JsonSummary {
    schema: &'static str,
    record: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp_unix_ms: Option<i64>,
    addr: std::net::SocketAddr,
    total_attempts: usize,
    successful_pings: usize,
    packet_loss: f64,
    min_duration_ms: f64,
    avg_duration_ms: f64,
    max_duration_ms: f64,
    resolve_time_ms: f64,
    jitter_p95_ms: Option<f64>,
}

impl From<&Summary> for JsonSummary {
    fn from(s: &Summary) -> Self {
        Self {
            schema: s.schema,
            record: s.record,
            timestamp: s.timestamp.as_ref().map(|ts| ts.rfc3339().to_string()),
            timestamp_unix_ms: s.timestamp.as_ref().map(RecordTimestamp::unix_ms),
            addr: s.addr,
            total_attempts: s.total_attempts,
            successful_pings: s.successful_pings,
            packet_loss: round2(s.packet_loss),
            min_duration_ms: round4(s.min_duration_ms),
            avg_duration_ms: round4(s.avg_duration_ms),
            max_duration_ms: round4(s.max_duration_ms),
            resolve_time_ms: round4(s.resolve_time_ms),
            jitter_p95_ms: s.jitter_p95_ms.map(round4),
        }
    }
}

pub struct Json;
impl Formatter for Json {
    fn probe(&self, res: &PingResult) {
        let out = JsonProbe::from(res);
        println!("{}", to_string(&out).expect("serialize JsonProbe"))
    }
    fn summary(&self, s: &Summary) {
        let out = JsonSummary::from(s);
        println!("{}", to_string(&out).expect("serialize JsonSummary"))
    }
}

/* ---------- CSV ---------- */

const CSV_HEADER_V1: &str = "record,address,status,rtt_ms,jitter_ms,total_attempts,successful_pings,packet_loss_pct,min_rtt_ms,avg_rtt_ms,max_rtt_ms,resolve_time_ms,jitter_p95_ms,schema";
const CSV_HEADER_V2: &str = "record,timestamp,timestamp_unix_ms,address,status,rtt_ms,jitter_ms,total_attempts,successful_pings,packet_loss_pct,min_rtt_ms,avg_rtt_ms,max_rtt_ms,resolve_time_ms,jitter_p95_ms,schema";
const CSV_COLUMNS_V1: usize = 14;
const CSV_COLUMNS_V2: usize = 16;

pub struct Csv {
    header_done: Cell<bool>,
    timestamps_enabled: bool,
}

impl Csv {
    pub fn new(timestamps_enabled: bool) -> Self {
        Self {
            header_done: Cell::new(false),
            timestamps_enabled,
        }
    }

    fn ensure_header(&self) {
        if !self.header_done.replace(true) {
            println!(
                "{}",
                if self.timestamps_enabled {
                    CSV_HEADER_V2
                } else {
                    CSV_HEADER_V1
                }
            );
        }
    }

    fn fmt_opt_ms(v: Option<f64>) -> String {
        v.map(|x| format!("{:.4}", x)).unwrap_or_default()
    }

    fn probe_row(res: &PingResult) -> String {
        let status = if res.success { "open" } else { "closed" };

        if let Some(timestamp) = res.timestamp.as_ref() {
            let mut fields = vec![String::new(); CSV_COLUMNS_V2];
            fields[0] = res.record.to_string();
            fields[1] = timestamp.rfc3339().to_string();
            fields[2] = timestamp.unix_ms().to_string();
            fields[3] = res.addr.to_string();
            fields[4] = status.to_string();
            fields[5] = format!("{:.4}", res.duration_ms);
            fields[6] = Self::fmt_opt_ms(res.jitter_ms);
            fields[15] = res.schema.to_string();
            fields.join(",")
        } else {
            let mut fields = vec![String::new(); CSV_COLUMNS_V1];
            fields[0] = res.record.to_string();
            fields[1] = res.addr.to_string();
            fields[2] = status.to_string();
            fields[3] = format!("{:.4}", res.duration_ms);
            fields[4] = Self::fmt_opt_ms(res.jitter_ms);
            fields[13] = res.schema.to_string();
            fields.join(",")
        }
    }

    fn summary_row(s: &Summary) -> String {
        if let Some(timestamp) = s.timestamp.as_ref() {
            let mut fields = vec![String::new(); CSV_COLUMNS_V2];
            fields[0] = s.record.to_string();
            fields[1] = timestamp.rfc3339().to_string();
            fields[2] = timestamp.unix_ms().to_string();
            fields[3] = s.addr.to_string();
            fields[7] = s.total_attempts.to_string();
            fields[8] = s.successful_pings.to_string();
            fields[9] = format!("{:.2}", s.packet_loss);
            fields[10] = format!("{:.4}", s.min_duration_ms);
            fields[11] = format!("{:.4}", s.avg_duration_ms);
            fields[12] = format!("{:.4}", s.max_duration_ms);
            fields[13] = format!("{:.4}", s.resolve_time_ms);
            fields[14] = Self::fmt_opt_ms(s.jitter_p95_ms);
            fields[15] = s.schema.to_string();
            fields.join(",")
        } else {
            let mut fields = vec![String::new(); CSV_COLUMNS_V1];
            fields[0] = s.record.to_string();
            fields[1] = s.addr.to_string();
            fields[5] = s.total_attempts.to_string();
            fields[6] = s.successful_pings.to_string();
            fields[7] = format!("{:.2}", s.packet_loss);
            fields[8] = format!("{:.4}", s.min_duration_ms);
            fields[9] = format!("{:.4}", s.avg_duration_ms);
            fields[10] = format!("{:.4}", s.max_duration_ms);
            fields[11] = format!("{:.4}", s.resolve_time_ms);
            fields[12] = Self::fmt_opt_ms(s.jitter_p95_ms);
            fields[13] = s.schema.to_string();
            fields.join(",")
        }
    }
}

impl Default for Csv {
    fn default() -> Self {
        Self::new(false)
    }
}
impl Formatter for Csv {
    fn probe(&self, res: &PingResult) {
        self.ensure_header();
        println!("{}", Self::probe_row(res));
    }

    fn summary(&self, s: &Summary) {
        self.ensure_header();
        println!("{}", Self::summary_row(s));
    }
}

/* ---------- Markdown table ---------- */

pub struct Md {
    header_done: Cell<bool>,
    timestamp_format: Option<TimestampFormat>,
}

impl Md {
    /// Construct a new Markdown formatter.
    pub fn new(timestamp_format: Option<TimestampFormat>) -> Self {
        Self {
            header_done: Cell::new(false),
            timestamp_format,
        }
    }

    fn render_row(&self, res: &PingResult) -> String {
        let status = if res.success { "ok" } else { "fail" };
        let jitter = res
            .jitter_ms
            .map(|j| format!("{:.4}", j))
            .unwrap_or_else(|| "-".into());
        match (res.timestamp.as_ref(), self.timestamp_format) {
            (Some(timestamp), Some(format)) => format!(
                "| {} | {} | {} | {:.4} | {} |",
                timestamp.render(format),
                res.addr,
                status,
                res.duration_ms,
                jitter
            ),
            _ => format!(
                "| {} | {} | {:.4} | {} |",
                res.addr, status, res.duration_ms, jitter
            ),
        }
    }
}

impl Default for Md {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Formatter for Md {
    fn probe(&self, res: &PingResult) {
        // print header once
        if !self.header_done.replace(true) {
            if self.timestamp_format.is_some() {
                println!("| timestamp | address | status | rtt_ms | jitter_ms |");
                println!("|-----------|---------|--------|--------|-----------|");
            } else {
                println!("| address | status | rtt_ms | jitter_ms |");
                println!("|---------|--------|--------|-----------|");
            }
        }

        println!("{}", self.render_row(res));
    }

    fn summary(&self, s: &Summary) {
        println!("\n### Summary\n");
        println!("| field | value |");
        println!("|-------|-------|");
        println!("| address | {} |", s.addr);
        if let (Some(timestamp), Some(format)) = (s.timestamp.as_ref(), self.timestamp_format) {
            println!("| timestamp | {} |", timestamp.render(format));
        }
        println!("| total probes | {} |", s.total_attempts);
        println!("| success | {} |", s.successful_pings);
        println!("| loss % | {:.2} |", s.packet_loss);
        println!(
            "| min / avg / max (ms) | {:.4} / {:.4} / {:.4} |",
            s.min_duration_ms, s.avg_duration_ms, s.max_duration_ms
        );
        println!("| resolve time (ms) | {:.4} |", s.resolve_time_ms);
        if let Some(j95) = s.jitter_p95_ms {
            println!("| jitter p95 (ms) | {:.4} |", j95);
        }
        println!();
    }
}

/* ---------- ANSI-colored TTY ---------- */

pub struct Color {
    timestamp_format: Option<TimestampFormat>,
}

impl Color {
    pub fn new(timestamp_format: Option<TimestampFormat>) -> Self {
        Self { timestamp_format }
    }

    fn render_probe(&self, res: &PingResult) -> String {
        let prefix = human_timestamp(res.timestamp.as_ref(), self.timestamp_format);
        let (status, color) = if res.success {
            ("open", "\x1b[32m") // green
        } else {
            ("closed", "\x1b[31m") // red
        };
        let reset = "\x1b[0m";
        match res.jitter_ms {
            Some(j) => format!(
                "{prefix}Probing {}/tcp - {color}{status}{reset} - {:.4} ms jitter={:.4} ms",
                res.addr, res.duration_ms, j
            ),
            None => format!(
                "{prefix}Probing {}/tcp - {color}{status}{reset} - {:.4} ms",
                res.addr, res.duration_ms
            ),
        }
    }
}

impl Formatter for Color {
    fn probe(&self, res: &PingResult) {
        println!("{}", self.render_probe(res));
    }

    fn summary(&self, s: &Summary) {
        let ok_color = "\x1b[32m";
        let bad_color = "\x1b[31m";
        let reset = "\x1b[0m";
        let prefix = human_timestamp(s.timestamp.as_ref(), self.timestamp_format);

        let color = if s.packet_loss == 0.0 {
            ok_color
        } else {
            bad_color
        };
        println!(
            "\n{prefix}--- {} tcping statistics ---\n\
{} probes sent, {} successful, {color}{:.2}%{reset} packet loss",
            s.addr, s.total_attempts, s.successful_pings, s.packet_loss
        );
        if s.successful_pings > 0 {
            println!(
                "Round-trip min/avg/max = {:.4}/{:.4}/{:.4} ms",
                s.min_duration_ms, s.avg_duration_ms, s.max_duration_ms
            );
        }
        if s.resolve_time_ms > 0.0 {
            println!("Address resolved in {:.4} ms", s.resolve_time_ms);
        }
        if let Some(j95) = s.jitter_p95_ms {
            println!("Jitter p95 = {:.4} ms", j95);
        }
    }
}

/* ---------- Factory ---------- */

pub fn from_mode(
    mode: OutputMode,
    timestamp_format: Option<TimestampFormat>,
) -> Box<dyn Formatter> {
    match mode {
        OutputMode::Normal => Box::new(Normal::new(timestamp_format)),
        OutputMode::Json => Box::new(Json),
        OutputMode::Csv => Box::new(Csv::new(timestamp_format.is_some())),
        OutputMode::Md => Box::new(Md::new(timestamp_format)),
        OutputMode::Color => Box::new(Color::new(timestamp_format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cli::TimestampFormat,
        stats::{OUTPUT_SCHEMA_V1, OUTPUT_SCHEMA_V2},
        timestamp::RecordTimestamp,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn sample_timestamp() -> RecordTimestamp {
        RecordTimestamp::from_unix_ms(1_746_072_812_345)
    }

    fn sample_result(
        success: bool,
        jitter: Option<f64>,
        timestamp: Option<RecordTimestamp>,
        schema: &'static str,
    ) -> PingResult {
        PingResult {
            schema,
            record: "probe",
            timestamp,
            success,
            duration_ms: 42.0,
            jitter_ms: jitter,
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80),
        }
    }

    fn sample_summary(
        jitter_p95: Option<f64>,
        timestamp: Option<RecordTimestamp>,
        schema: &'static str,
    ) -> Summary {
        Summary {
            schema,
            record: "summary",
            timestamp,
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 80),
            total_attempts: 4,
            successful_pings: 3,
            packet_loss: 25.0,
            min_duration_ms: 1.0,
            avg_duration_ms: 2.0,
            max_duration_ms: 3.0,
            resolve_time_ms: 0.5,
            jitter_p95_ms: jitter_p95,
        }
    }

    #[test]
    fn markdown_header_only_prints_once() {
        let fmt = Md::new(None);
        assert!(!fmt.header_done.get());
        fmt.probe(&sample_result(true, None, None, OUTPUT_SCHEMA_V1));
        assert!(fmt.header_done.get());
        fmt.probe(&sample_result(true, None, None, OUTPUT_SCHEMA_V1));
        assert!(fmt.header_done.get());
    }

    #[test]
    fn markdown_rows_use_ascii_status() {
        let fmt = Md::new(None);
        let ok_row = fmt.render_row(&sample_result(true, Some(1.5), None, OUTPUT_SCHEMA_V1));
        assert!(ok_row.contains("| ok |"));
        assert!(ok_row.contains("1.5000"));

        let fail_row = fmt.render_row(&sample_result(false, None, None, OUTPUT_SCHEMA_V1));
        assert!(fail_row.contains("| fail |"));
        assert!(fail_row.ends_with(" | - |") || fail_row.contains("| - |"));
    }

    #[test]
    fn markdown_rows_include_timestamp_when_enabled() {
        let fmt = Md::new(Some(TimestampFormat::Iso8601));
        let row = fmt.render_row(&sample_result(
            true,
            Some(1.5),
            Some(sample_timestamp()),
            OUTPUT_SCHEMA_V2,
        ));
        assert!(row.contains("2025-05-01T04:13:32.345Z"));
        assert!(row.contains("| ok |"));
    }

    #[test]
    fn csv_rows_match_header_column_count() {
        assert_eq!(CSV_HEADER_V1.split(',').count(), CSV_COLUMNS_V1);
        assert_eq!(CSV_HEADER_V2.split(',').count(), CSV_COLUMNS_V2);

        let probe_row = Csv::probe_row(&sample_result(true, None, None, OUTPUT_SCHEMA_V1));
        assert_eq!(probe_row.split(',').count(), CSV_COLUMNS_V1);
        assert_eq!(probe_row.split(',').next_back(), Some(OUTPUT_SCHEMA_V1));

        let probe_row = Csv::probe_row(&sample_result(
            true,
            Some(1.5),
            Some(sample_timestamp()),
            OUTPUT_SCHEMA_V2,
        ));
        assert_eq!(probe_row.split(',').count(), CSV_COLUMNS_V2);
        assert_eq!(probe_row.split(',').next_back(), Some(OUTPUT_SCHEMA_V2));

        let summary_row = Csv::summary_row(&sample_summary(
            Some(1.23),
            Some(sample_timestamp()),
            OUTPUT_SCHEMA_V2,
        ));
        assert_eq!(summary_row.split(',').count(), CSV_COLUMNS_V2);
        assert_eq!(summary_row.split(',').next_back(), Some(OUTPUT_SCHEMA_V2));
    }

    #[test]
    fn csv_summary_columns_are_aligned() {
        let row = Csv::summary_row(&sample_summary(None, None, OUTPUT_SCHEMA_V1));
        let cols: Vec<&str> = row.split(',').collect();
        assert_eq!(cols.len(), CSV_COLUMNS_V1);
        assert_eq!(cols[0], "summary");
        assert_eq!(cols[1], "127.0.0.1:80");
        assert_eq!(cols[5], "4");
        assert_eq!(cols[6], "3");
        assert_eq!(cols[7], "25.00");
        assert_eq!(cols[13], OUTPUT_SCHEMA_V1);
    }

    #[test]
    fn json_includes_timestamp_fields_when_present() {
        let probe = JsonProbe::from(&sample_result(
            true,
            None,
            Some(sample_timestamp()),
            OUTPUT_SCHEMA_V2,
        ));
        let json = to_string(&probe).expect("serialize JsonProbe");
        assert!(json.contains("\"timestamp\":\"2025-05-01T04:13:32.345Z\""));
        assert!(json.contains("\"timestamp_unix_ms\":1746072812345"));
    }

    #[test]
    fn json_omits_timestamp_fields_when_disabled() {
        let summary = JsonSummary::from(&sample_summary(None, None, OUTPUT_SCHEMA_V1));
        let json = to_string(&summary).expect("serialize JsonSummary");
        assert!(!json.contains("timestamp_unix_ms"));
        assert!(!json.contains("\"timestamp\""));
    }
}
