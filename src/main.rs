//! tcping-rs — Async implementation based on Tokio.
//!
//! The original blocking `TcpStream::connect_timeout` is replaced by
//! an asynchronous combination of **Tokio + `timeout` + `select!`**.
//! All CLI flags, output formats, statistics logic and unit tests
//! remain intact so you can drop‑in replace your previous version.

use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::Instant,
};
use tokio::{
    net::TcpStream,
    signal,
    time::{Duration, Instant as TokioInstant, sleep, timeout},
};

/// CLI arguments parsed by `clap`.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target in the form `<host:port>`
    address: String,

    /// Number of probes to send (`-c`)
    #[arg(short = 'c', long, default_value_t = 4)]
    count: usize,

    /// Keep probing until Ctrl‑C (`-t`)
    #[arg(short = 't', long)]
    continuous: bool,

    /// Output format: *normal*, *json* or *csv* (`-o`)
    #[arg(short = 'o', long, value_enum, default_value_t = OutputMode::Normal)]
    output_mode: OutputMode,

    /// Stop immediately after the first successful probe (`-e`)
    #[arg(short = 'e', long)]
    exit_on_success: bool,

    /// Calculate and print per‑probe jitter (`-j`)
    #[arg(short = 'j', long)]
    jitter: bool,

    /// Per‑probe timeout in **milliseconds** (`--timeout-ms`)
    #[arg(long, default_value_t = 2000)]
    timeout_ms: u64,
}

/// Supported output modes.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Normal,
    Json,
    Csv,
}

/// Result of a single probe, serialisable to JSON/CSV.
#[derive(Serialize, Clone)]
struct PingResult {
    success: bool,
    duration_ms: f64,
    jitter_ms: Option<f64>,
    addr: SocketAddr,
}

/// Final aggregated statistics.
#[derive(Serialize)]
struct Summary {
    addr: SocketAddr,
    total_attempts: usize,
    successful_pings: usize,
    packet_loss: f64,
    min_duration_ms: f64,
    avg_duration_ms: f64,
    max_duration_ms: f64,
    resolve_time_ms: f64,
}

/// Conduct a single asynchronous TCP probe with timeout.
async fn probe_once(addr: SocketAddr, to: Duration) -> (bool, f64) {
    let start = TokioInstant::now();
    match timeout(to, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => (true, start.elapsed().as_secs_f64() * 1_000.0),
        _ => (false, to.as_secs_f64() * 1_000.0),
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    /* ───────────── Address resolution ───────────── */
    let resolve_start = Instant::now();
    let addr = match args.address.to_socket_addrs() {
        Ok(mut addrs) => addrs.next().expect("failed to resolve hostname"),
        Err(_) => {
            eprintln!("Error: invalid address");
            return;
        }
    };
    if addr.port() == 0 {
        eprintln!("Error: port cannot be 0");
        return;
    }
    let resolve_time = resolve_start.elapsed().as_secs_f64() * 1_000.0;
    let timeout = Duration::from_millis(args.timeout_ms);

    if args.continuous && matches!(args.output_mode, OutputMode::Normal) {
        println!("\n** Probing continuously — Ctrl‑C to stop **");
    }
    if matches!(args.output_mode, OutputMode::Normal) {
        println!("\nResolved address in {:.4} ms", resolve_time);
    }

    /* ───────────── Aggregation variables ───────────── */
    let mut success = 0usize;
    let mut total_rtt = 0f64;
    let mut min_rtt = f64::MAX;
    let mut max_rtt = 0f64;
    let mut sent = 0usize;
    let mut results: Vec<PingResult> = Vec::new();

    // Ctrl‑C future
    let sigint = signal::ctrl_c();
    tokio::pin!(sigint);

    /* ───────────── Main probe loop ───────────── */
    loop {
        if !args.continuous && sent >= args.count {
            break;
        }

        sent += 1;
        let (ok, rtt) = probe_once(addr, timeout).await;

        if ok {
            // success path
            success += 1;
            min_rtt = min_rtt.min(rtt);
            max_rtt = max_rtt.max(rtt);
            total_rtt += rtt;

            let avg = total_rtt / success as f64;
            let jitter = args.jitter.then(|| (rtt - avg).abs());

            let res = PingResult {
                success: true,
                duration_ms: rtt,
                jitter_ms: jitter,
                addr,
            };
            results.push(res.clone());

            match args.output_mode {
                OutputMode::Normal => {
                    if let Some(j) = jitter {
                        println!("Probing {addr}/tcp - open - {rtt:.4} ms jitter={j:.4} ms");
                    } else {
                        println!("Probing {addr}/tcp - open - {rtt:.4} ms");
                    }
                }
                OutputMode::Json => println!("{}", serde_json::to_string(&res).unwrap()),
                OutputMode::Csv => {
                    if let Some(j) = jitter {
                        println!("{addr},open,{rtt:.4},{j:.4}");
                    } else {
                        println!("{addr},open,{rtt:.4}");
                    }
                }
            }

            if args.exit_on_success {
                break;
            }
        } else {
            // failure / timeout path
            let res = PingResult {
                success: false,
                duration_ms: rtt,
                jitter_ms: None,
                addr,
            };
            results.push(res.clone());

            match args.output_mode {
                OutputMode::Normal => println!("Probing {addr}/tcp - closed - {rtt:.4} ms"),
                OutputMode::Json => println!("{}", serde_json::to_string(&res).unwrap()),
                OutputMode::Csv => println!("{addr},closed,{rtt:.4}"),
            }
        }

        if !args.continuous && sent >= args.count {
            break;
        }

        // wait 1 s or until Ctrl‑C is received
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {},
            _ = &mut sigint => break,
        }
    }

    /* ───────────── Summary output ───────────── */
    let summary = Summary {
        addr,
        total_attempts: sent,
        successful_pings: success,
        packet_loss: 100.0 * (1.0 - success as f64 / sent as f64),
        min_duration_ms: if success > 0 { min_rtt } else { 0.0 },
        avg_duration_ms: if success > 0 {
            total_rtt / success as f64
        } else {
            0.0
        },
        max_duration_ms: if success > 0 { max_rtt } else { 0.0 },
        resolve_time_ms: resolve_time,
    };

    match args.output_mode {
        OutputMode::Normal => {
            println!(
                "\n--- {} tcping statistics ---
{} probes sent, {} successful, {:.2}% packet loss",
                summary.addr, summary.total_attempts, summary.successful_pings, summary.packet_loss
            );
            if summary.successful_pings > 0 {
                println!(
                    "Round‑trip  min/avg/max = {:.4}/{:.4}/{:.4} ms",
                    summary.min_duration_ms, summary.avg_duration_ms, summary.max_duration_ms
                );
            }
            println!("Address resolved in {:.4} ms\n", summary.resolve_time_ms);
        }
        OutputMode::Json => println!("{}", serde_json::to_string(&summary).unwrap()),
        OutputMode::Csv => println!(
            "address,total_probes,successful_probes,packet_loss,min_rtt,avg_rtt,max_rtt,resolve_time_ms\n\
             {addr},{sent},{success},{:.2},{:.4},{:.4},{:.4},{:.4}",
            summary.packet_loss,
            summary.min_duration_ms,
            summary.avg_duration_ms,
            summary.max_duration_ms,
            summary.resolve_time_ms
        ),
    }
}

/* ───────────── Unit tests ───────────── */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_flags() {
        let args = Args::parse_from(["tcping", "127.0.0.1:80", "-c", "5"]);
        assert_eq!(args.address, "127.0.0.1:80");
        assert_eq!(args.count, 5);
        assert!(!args.continuous);
        assert_eq!(args.output_mode, OutputMode::Normal);
        assert!(!args.exit_on_success);
        assert!(!args.jitter);
    }

    #[test]
    fn parse_continuous() {
        let args = Args::parse_from(["tcping", "127.0.0.1:80", "-t"]);
        assert!(args.continuous);
    }

    #[test]
    fn resolve_domain() {
        assert!("example.com:80".to_socket_addrs().is_ok());
    }

    #[test]
    fn parse_output_mode() {
        let args = Args::parse_from(["tcping", "127.0.0.1:80", "-o", "json"]);
        assert_eq!(args.output_mode, OutputMode::Json);
    }

    #[test]
    fn parse_exit_on_success() {
        let args = Args::parse_from(["tcping", "127.0.0.1:80", "-e"]);
        assert!(args.exit_on_success);
    }

    #[test]
    fn parse_jitter() {
        let args = Args::parse_from(["tcping", "127.0.0.1:80", "-j"]);
        assert!(args.jitter);
    }
}
