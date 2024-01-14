use clap::Parser;
use std::{
    net::{TcpStream, ToSocketAddrs},
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

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

    // Validate and resolve the address
    let addr = match args.address.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => {
                // println!("Resolved IP: {}", addr.ip());
                addr
            }
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

    if args.continuous {
        println!();
        println!("** Pinging continuously.  Press control-c to stop **");
        println!();
    }

    let mut successful_pings = 0;
    let mut total_duration = 0f64;
    let mut min_duration = f64::MAX;
    let mut max_duration = 0f64;
    let mut total_attempts = 0;

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
                println!(
                    "Probing {}/tcp - Port is open - time={:.4}ms",
                    addr, duration
                );
            }
            Err(_) => {
                let duration = timeout.as_micros() as f64 / 1000.0;
                println!(
                    "Probing {}/tcp - No response - time={:.4}ms",
                    addr, duration
                );
            }
        }

        thread::sleep(Duration::from_secs(1));
    }

    let avg_duration = if successful_pings > 0 {
        total_duration / successful_pings as f64
    } else {
        0.0
    };

    println!("\n--- {} tcping statistics ---", addr);
    println!(
        "{} probes sent, {} successful, {:.2}% packet loss",
        total_attempts,
        successful_pings,
        100.0 * (1.0 - (successful_pings as f64 / total_attempts as f64))
    );

    if successful_pings > 0 {
        println!(
            "Round-trip min/avg/max = {:.4}ms/{:.4}ms/{:.4}ms",
            min_duration, avg_duration, max_duration
        );
    }
}
