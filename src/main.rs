use chrono::prelude::*;
use clap::Parser;
use std::{
    io,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
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

    /// output format
    #[clap(short = 'o', long, default_value_t = String::from("text"))]
    output: String,
}

fn local_time<'a>() -> chrono::format::DelayedFormat<chrono::format::StrftimeItems<'a>> {
    let local: DateTime<Local> = Local::now();
    local.format("%Y-%m-%d %H:%M:%S")
}
enum Output {
    Text,
    Json,
    Csv,
}
impl Output {
    fn eprint(&self, e: &str) {
        match self {
            Output::Text => eprintln!("Error: {}", e),
            Output::Json => println!(
                "{{\"type\":\"error\",\"time\":\"{}\",\"error\":\"{}\"}}",
                local_time(),
                e
            ),
            Output::Csv => println!("error,{},{}", local_time(), e),
        }
    }
    fn newline(&self) {
        match self {
            Output::Text => println!(),
            Output::Json => (),
            Output::Csv => (),
        }
    }
    fn probing(&self, addr: &SocketAddr, duration: f64) {
        match self {
            Output::Text => println!(
                "Probing {}/tcp - Port is open - time={:.3} ms",
                addr, duration
            ),
            Output::Json => {
                println!(
                "{{\"type\":\"probing\",\"time\":\"{}\",\"address\":\"{}\",\"duration\":{:.3}}}",
                local_time(),addr, duration
            )
            }
            Output::Csv => println!("probing,{},{},{:.3}", local_time(), addr, duration),
        }
    }
    fn eprobing(&self, addr: &SocketAddr, e: io::Error) {
        match self {
            Output::Text => println!("Probing {}/tcp - error: {}", addr, e),
            Output::Json => println!(
                "{{\"type\":\"error\",\"time\":\"{}\",\"address\":\"{}\",\"error\":\"{}\"}}",
                local_time(),
                addr,
                e
            ),
            Output::Csv => println!("error,{},{},{}", local_time(), addr, e),
        }
    }
    fn print(&self, s: &str) {
        match self {
            Output::Text => println!("{}", s),
            Output::Json => (),
            Output::Csv => (),
        }
    }
    fn stat1(&self, total_attempts: usize, successful_pings: usize) {
        let pect = 100.0 * (1.0 - (successful_pings as f64 / total_attempts as f64));
        match self {
            Output::Text => println!(
                "{} probes sent, {} successful, {:.2}% packet loss",
                total_attempts, successful_pings, pect
            ),
            Output::Json => println!(
                "{{\"type\":\"statistics\",\"time\":\"{}\",\"total_attempts\":{},\"successful_pings\":{},\"loss_percent\":{:.2}}}",
                local_time(),total_attempts, successful_pings,pect
            ),
            Output::Csv => println!("statistics,{},{},{},{:.2}", local_time(), total_attempts, successful_pings,pect),
        }
    }
    fn stat2(&self, min_duration: f64, avg_duration: f64, max_duration: f64) {
        match self {
            Output::Text => println!(
                "Round-trip min/avg/max = {:.3}/{:.3}/{:.3} ms",
                min_duration, avg_duration, max_duration
            ),
            Output::Json => println!(
                "{{\"type\":\"roundtrip\",\"time\":\"{}\",\"min\":{:.3},\"avg\":{:.3},\"max\":{:.3}}}",
                local_time(),min_duration,avg_duration,max_duration
            ),
            Output::Csv => println!("roundtrip,{},{:.3},{:.3},{:.3}", local_time(), min_duration, avg_duration,max_duration),
        }
    }
}

fn main() {
    let args = Args::parse();

    let output = match args.output.as_str() {
        "text" => Output::Text,
        "json" => Output::Json,
        "csv" => Output::Csv,
        _ => panic!("unsupported output format"),
    };

    // Validate and resolve the address
    let addr = match args.address.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => {
                // println!("Resolved IP: {}", addr.ip());
                addr
            }
            None => {
                output.eprint("Unable to resolve address");
                return;
            }
        },
        Err(_) => {
            output.eprint("Invalid address format");
            return;
        }
    };

    // Validate the port
    if addr.port() == 0 {
        output.eprint("Invalid port number");
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
        output.newline();
        eprintln!("** Pinging continuously.  Press control-c to stop **");
    }
    output.newline();

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
                output.probing(&addr, duration);
            }
            Err(e) => {
                output.eprobing(&addr, e);
            }
        }

        thread::sleep(Duration::from_secs(1));
    }

    let avg_duration = if successful_pings > 0 {
        total_duration / successful_pings as f64
    } else {
        0.0
    };

    output.print(format!("\n--- {} tcping statistics ---", addr).as_str());
    output.stat1(total_attempts, successful_pings);

    if successful_pings > 0 {
        output.stat2(min_duration, avg_duration, max_duration);
    }
    output.newline();
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
    }

    #[test]
    fn test_args_parsing_with_continuous() {
        let args = Args::parse_from("tcping 127.0.0.1:80 -t".split_whitespace());
        assert_eq!(args.address, "127.0.0.1:80");
        assert_eq!(args.count, 4);
        assert!(args.continuous);
    }

    #[test]
    fn test_domain_name_resolution() {
        let domain_name = "cloudflare.com:80";
        let socket_addrs = domain_name.to_socket_addrs();
        assert!(socket_addrs.is_ok(), "Failed to resolve domain name");
    }
}
