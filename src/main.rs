use clap::{Parser, ValueEnum};
use serde::Serialize;
use std::{
    net::{TcpStream, ToSocketAddrs},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

/// TCP ping tool to measure the latency to a server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The address to ping in the format <host:port>
    address: String,

    /// Number of pings to send
    #[arg(short = 'c', long, default_value_t = 4)]
    count: usize,

    /// Ping continuously until interrupted
    #[arg(short = 't', long)]
    continuous: bool,

    /// Output mode: normal, json, or csv
    #[arg(short = 'o', long, value_enum, default_value_t = OutputMode::Normal)]
    output_mode: OutputMode,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum OutputMode {
    Normal,
    Json,
    Csv,
}

#[derive(Serialize, Clone)]
struct PingResult {
    success: bool,
    duration_ms: f64,
    addr: std::net::SocketAddr,
}

#[derive(Serialize)]
struct Summary {
    addr: std::net::SocketAddr,
    total_attempts: usize,
    successful_pings: usize,
    packet_loss: f64,
    min_duration_ms: f64,
    avg_duration_ms: f64,
    max_duration_ms: f64,
}

fn main() {
    let args = Args::parse();

    // Validate and resolve the address
    let addr = match args.address.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => {
                eprintln!("Error: Unable to resolve address");
                return;
            }
        },
        Err(_) => {
            eprintln!("Error: Invalid address format");
            return;
        }
    };

    // Validate the port
    if addr.port() == 0 {
        eprintln!("Error: Invalid port number");
        return;
    }

    let timeout = Duration::new(2, 0);
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if args.continuous && args.output_mode == OutputMode::Normal {
        println!();
        println!("** Pinging continuously. Press control-c to stop **");
    }
    if args.output_mode == OutputMode::Normal {
        println!();
    }

    let mut successful_pings = 0;
    let mut total_duration = 0f64;
    let mut min_duration = f64::MAX;
    let mut max_duration = 0f64;
    let mut total_attempts = 0;
    let mut results = Vec::new();

    while running.load(Ordering::SeqCst) && (args.continuous || total_attempts < args.count) {
        total_attempts += 1;

        let start = Instant::now();
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => {
                let duration = start.elapsed().as_micros() as f64 / 1000.0;
                min_duration = min_duration.min(duration);
                max_duration = max_duration.max(duration);
                total_duration += duration;
                successful_pings += 1;

                let result = PingResult {
                    success: true,
                    duration_ms: duration,
                    addr,
                };
                results.push(result.clone());

                match args.output_mode {
                    OutputMode::Normal => {
                        println!(
                            "Probing {}/tcp - Port is open - time={:.4}ms",
                            addr, duration
                        );
                    }
                    OutputMode::Json => {
                        let json = serde_json::to_string(&result).unwrap();
                        println!("{}", json);
                    }
                    OutputMode::Csv => {
                        println!("{},{},{:.4}", addr, "open", duration);
                    }
                }
            }
            Err(_) => {
                let duration = timeout.as_micros() as f64 / 1000.0;

                let result = PingResult {
                    success: false,
                    duration_ms: duration,
                    addr,
                };
                results.push(result.clone());

                match args.output_mode {
                    OutputMode::Normal => {
                        println!(
                            "Probing {}/tcp - No response - time={:.4}ms",
                            addr, duration
                        );
                    }
                    OutputMode::Json => {
                        let json = serde_json::to_string(&result).unwrap();
                        println!("{}", json);
                    }
                    OutputMode::Csv => {
                        println!("{},{},{:.4}", addr, "closed", duration);
                    }
                }
            }
        }

        thread::sleep(Duration::from_secs(1));
    }

    let avg_duration = if successful_pings > 0 {
        total_duration / successful_pings as f64
    } else {
        0.0
    };

    let packet_loss = 100.0 * (1.0 - (successful_pings as f64 / total_attempts as f64));

    let summary = Summary {
        addr,
        total_attempts,
        successful_pings,
        packet_loss,
        min_duration_ms: if successful_pings > 0 {
            min_duration
        } else {
            0.0
        },
        avg_duration_ms: avg_duration,
        max_duration_ms: if successful_pings > 0 {
            max_duration
        } else {
            0.0
        },
    };

    match args.output_mode {
        OutputMode::Normal => {
            println!("\n--- {} tcping statistics ---", addr);
            println!(
                "{} probes sent, {} successful, {:.2}% packet loss",
                total_attempts, successful_pings, packet_loss
            );
            if successful_pings > 0 {
                println!(
                    "Round-trip min/avg/max = {:.4}ms/{:.4}ms/{:.4}ms",
                    min_duration, avg_duration, max_duration
                );
            }
            println!();
        }
        OutputMode::Json => {
            let json = serde_json::to_string(&summary).unwrap();
            println!("{}", json);
        }
        OutputMode::Csv => {
            println!("address,total_probes,successful_probes,packet_loss,min_rtt,avg_rtt,max_rtt");
            println!(
                "{},{},{},{:.2},{:.4},{:.4},{:.4}",
                addr,
                total_attempts,
                successful_pings,
                packet_loss,
                summary.min_duration_ms,
                summary.avg_duration_ms,
                summary.max_duration_ms
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::ToSocketAddrs;

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from("tcping 127.0.0.1:80 -c 5".split_whitespace());
        assert_eq!(args.address, "127.0.0.1:80");
        assert_eq!(args.count, 5);
        assert!(!args.continuous);
        assert_eq!(args.output_mode, OutputMode::Normal);
    }

    #[test]
    fn test_args_parsing_with_continuous() {
        let args = Args::parse_from("tcping 127.0.0.1:80 -t".split_whitespace());
        assert_eq!(args.address, "127.0.0.1:80");
        assert_eq!(args.count, 4);
        assert!(args.continuous);
        assert_eq!(args.output_mode, OutputMode::Normal);
    }

    #[test]
    fn test_domain_name_resolution() {
        let domain_name = "cloudflare.com:80";
        let socket_addrs = domain_name.to_socket_addrs();
        assert!(socket_addrs.is_ok(), "Failed to resolve domain name");
    }

    #[test]
    fn test_output_mode_parsing() {
        let args = Args::parse_from("tcping 127.0.0.1:80 -o json".split_whitespace());
        assert_eq!(args.output_mode, OutputMode::Json);
    }
}
