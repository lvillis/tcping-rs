//! Orchestrator: resolve targets, run fixed-interval probes, and emit formatted output.
//!
//! Features:
//! - Two-thread Tokio runtime (I/O + timer).
//! - Windows-specific boosts in `main.rs` raise timer resolution and thread priority.
//! - The first `tokio::time::interval` tick is consumed to avoid double-printing.
//! - Domain targets print a resolution banner that includes DNS server details.
use crate::{
    cli::{Args, OutputMode},
    error::{Result, TcpingError},
    formatter::{self, Formatter},
    probe::probe_once,
    stats::Stats,
};
use std::{
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    time::Instant,
};
use tokio::{signal, time};

struct ParsedTarget {
    host: String,
    port: u16,
    is_literal: bool,
}

impl ParsedTarget {
    fn new(address: &str) -> Result<Self> {
        let trimmed = address.trim();
        let (host_part, port_part) = if let Some(rest) = trimmed.strip_prefix('[') {
            let closing = rest.find(']').ok_or_else(|| {
                TcpingError::Other(anyhow::anyhow!("missing closing ']' in target"))
            })?;
            let host = &rest[..closing];
            let remainder = rest[closing + 1..].strip_prefix(':').ok_or_else(|| {
                TcpingError::Other(anyhow::anyhow!("IPv6 target must end with ]:port"))
            })?;
            (host, remainder)
        } else {
            trimmed.rsplit_once(':').ok_or_else(|| {
                TcpingError::Other(anyhow::anyhow!("target must be in the form host:port"))
            })?
        };

        if host_part.is_empty() {
            return Err(TcpingError::Other(anyhow::anyhow!("target host is empty")));
        }

        let port: u16 = port_part.parse().map_err(|_| {
            TcpingError::Other(anyhow::anyhow!(
                "port must be an integer between 0 and 65535"
            ))
        })?;

        let is_literal = host_part.parse::<IpAddr>().is_ok();

        Ok(Self {
            host: host_part.to_string(),
            port,
            is_literal,
        })
    }

    fn socket_addr(&self) -> Option<SocketAddr> {
        self.host
            .parse::<IpAddr>()
            .ok()
            .map(|ip| SocketAddr::new(ip, self.port))
    }
}

/// Best-effort reader for the first DNS server listed in `/etc/resolv.conf`
/// (Unix only). Returns `None` on Windows or failure.
fn first_dns_server() -> Option<IpAddr> {
    #[cfg(unix)]
    {
        use std::fs::File;
        use std::io::{BufRead, BufReader};
        let file = File::open("/etc/resolv.conf").ok()?;
        for line in BufReader::new(file).lines().flatten() {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("nameserver") {
                return rest.trim().parse().ok();
            }
        }
    }
    None
}

/// Create a two-thread Tokio runtime and block on the async runner.
pub fn run(args: Args) -> Result<i32> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(run_async(args))
}

/// Async tcping session.
pub async fn run_async(args: Args) -> Result<i32> {
    /* ---------- target parsing ---------- */
    let parsed = ParsedTarget::new(&args.address)?;

    /* ---------- resolution ---------- */
    let (addr, resolve_ms) = if parsed.is_literal {
        let addr = parsed
            .socket_addr()
            .ok_or_else(|| TcpingError::Other(anyhow::anyhow!("invalid IP literal")))?;
        (addr, 0.0)
    } else {
        let start = Instant::now();
        let mut iter = (parsed.host.as_str(), parsed.port).to_socket_addrs()?;
        let addr = iter
            .next()
            .ok_or_else(|| TcpingError::Other(anyhow::anyhow!("unresolvable host")))?;
        let ms = start.elapsed().as_secs_f64() * 1_000.0;

        if matches!(args.output_mode, OutputMode::Normal) {
            let dns = first_dns_server()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "system default".into());
            println!(
                "\nResolved {} -> {}  (DNS {dns})  in {:.4} ms\n",
                parsed.host,
                addr.ip(),
                ms
            );
        }
        (addr, ms)
    };

    /* ---------- blank line for IP literal ---------- */
    if parsed.is_literal && matches!(args.output_mode, OutputMode::Normal) {
        println!();
    }

    /* ---------- stats & formatter ---------- */
    let mut stats = Stats::new(addr, resolve_ms);
    let fmt: Box<dyn Formatter> = formatter::from_mode(args.output_mode);

    /* ---------- Ctrl-C future ---------- */
    let sigint = signal::ctrl_c();
    tokio::pin!(sigint);

    /* ---------- 1-second ticker ---------- */
    let mut ticker = time::interval(time::Duration::from_secs(1));
    ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    ticker.tick().await; // consume zero-delay first tick

    let timeout = time::Duration::from_millis(args.timeout_ms);
    let mut first = true;

    loop {
        /* ---- probe ---- */
        if !first {
            tokio::select! {
                _ = ticker.tick() => {},
                _ = &mut sigint   => break,
            }
        } else {
            first = false; // skip waiting before probe #1
        }

        let (ok, rtt) = probe_once(addr, timeout).await;
        let res = stats.feed(ok, rtt, args.jitter);
        fmt.probe(&res);

        if stats.should_break(ok, &args) || !stats.should_continue(&args) {
            break;
        }
    }

    /* ---------- summary ---------- */
    fmt.summary(&stats.summary());
    Ok(stats.exit_code())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_literal() {
        let parsed = ParsedTarget::new("127.0.0.1:80").unwrap();
        assert!(parsed.is_literal);
        assert_eq!(parsed.host, "127.0.0.1");
        assert_eq!(parsed.port, 80);
        assert_eq!(parsed.socket_addr(), Some("127.0.0.1:80".parse().unwrap()));
    }

    #[test]
    fn parses_ipv6_literal() {
        let parsed = ParsedTarget::new("[::1]:443").unwrap();
        assert!(parsed.is_literal);
        assert_eq!(parsed.host, "::1");
        assert_eq!(parsed.port, 443);
        assert_eq!(parsed.socket_addr(), Some("[::1]:443".parse().unwrap()));
    }

    #[test]
    fn parses_hostname() {
        let parsed = ParsedTarget::new("example.com:9000").unwrap();
        assert!(!parsed.is_literal);
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 9000);
        assert!(parsed.socket_addr().is_none());
    }

    #[test]
    fn rejects_missing_port() {
        assert!(ParsedTarget::new("example.com").is_err());
    }
}
