use clap::Parser;
use std::{net::TcpStream, sync::atomic::{AtomicBool, Ordering}, sync::Arc, time::{Duration, Instant}};

/// TCP ping tool to measure the latency to a server
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The address to ping in the format <host:port>
    #[clap(value_parser)]
    address: String,

    /// Number of pings to send
    #[clap(short = 'c', long, default_value_t = 4)]
    count: usize,

    /// Ping continuously until interrupted
    #[clap(short = 't', long)]
    continuous: bool,
}

fn main() {
    let args = Args::parse();

    let timeout = Duration::new(2, 0);
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    if args.continuous {
        println!("** Pinging continuously.  Press control-c to stop **");
    }

    let mut successful_pings = 0;
    let mut total_duration = 0f64;
    let mut min_duration = f64::MAX;
    let mut max_duration = 0f64;
    let mut total_count = args.count;

    while running.load(Ordering::SeqCst) && (args.continuous || total_count > 0) {
        let start = Instant::now();
        match TcpStream::connect_timeout(&args.address.parse().unwrap(), timeout) {
            Ok(_) => {
                let duration = start.elapsed().as_micros() as f64 / 1000.0;
                min_duration = min_duration.min(duration);
                max_duration = max_duration.max(duration);
                total_duration += duration;
                successful_pings += 1;
                println!("Probing {}/tcp - Port is open - time={:.4}ms", args.address, duration);
            }
            Err(_) => {
                let duration = timeout.as_micros() as f64 / 1000.0;
                println!("Probing {}/tcp - No response - time={:.4}ms", args.address, duration);
            }
        }

        if !args.continuous {
            total_count -= 1;
        }
    }

    let total_attempts = if args.continuous { successful_pings } else { args.count };
    let avg_duration = if successful_pings > 0 { total_duration / successful_pings as f64 } else { 0.0 };

    println!("\n--- {} tcping statistics ---", args.address);
    println!("{} probes sent, {} successful, {:.2}% packet loss", total_attempts, successful_pings,
             100.0 * (1.0 - (successful_pings as f64 / total_attempts as f64)));

    if successful_pings > 0 {
        println!("Round-trip min/avg/max = {:.4}ms/{:.4}ms/{:.4}ms", min_duration, avg_duration, max_duration);
    }
}
