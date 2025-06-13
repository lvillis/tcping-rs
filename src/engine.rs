//! High-level orchestration: resolve → probe loop → formatted output.
//!
//! * [`run`]      — synchronous wrapper that owns its own Tokio runtime.
//! * [`run_async`] — async API for callers already inside a runtime.
//!
//! For a **domain-name** target we print a “Resolved …” line with the
//! resolved IP and primary DNS server, followed by a blank line.
//! For an **IP literal** target we skip DNS resolution entirely but still
//! emit a leading blank line so the first `Probing …` line does not stick
//! to the prompt.

use crate::{
    cli::{Args, OutputMode},
    error::{Result, TcpingError},
    formatter::{self, Formatter},
    probe::probe_once,
    stats::Stats,
};
use std::{
    net::{IpAddr, ToSocketAddrs},
    time::Instant,
};
use tokio::{
    signal,
    time::{sleep, Duration},
};

/// Return the first DNS server declared in `/etc/resolv.conf` (Unix only).
/// On non-Unix platforms or failure it returns `None`.
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

/// Synchronous helper that spins up a single-thread Tokio runtime.
pub fn run(args: Args) -> Result<i32> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(run_async(args))
}

/// Full tcping session (async variant).
pub async fn run_async(args: Args) -> Result<i32> {
    // Host / port split
    let (host, _port) = args
        .address
        .split_once(':')
        .ok_or_else(|| TcpingError::Other(anyhow::anyhow!("target must be host:port")))?;

    let is_ip_literal = host.parse::<IpAddr>().is_ok();

    // DNS resolution (domains only)
    let (addr, resolve_ms) = if is_ip_literal {
        let addr = args
            .address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| TcpingError::Other(anyhow::anyhow!("invalid address")))?;
        (addr, 0.0)
    } else {
        let start = Instant::now();
        let addr = args
            .address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| TcpingError::Other(anyhow::anyhow!("unresolvable host")))?;
        let ms = start.elapsed().as_secs_f64() * 1_000.0;

        if matches!(args.output_mode, OutputMode::Normal) {
            let dns = first_dns_server()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "system default".into());

            println!(
                "\nResolved {host} → {}  (DNS {dns})  in {:.4} ms\n",
                addr.ip(),
                ms
            );
        }
        (addr, ms)
    };

    // IP literal still needs a leading blank line
    if is_ip_literal && matches!(args.output_mode, OutputMode::Normal) {
        println!();
    }

    let timeout = Duration::from_millis(args.timeout_ms);

    if args.continuous && matches!(args.output_mode, OutputMode::Normal) {
        println!("** Probing continuously — press Ctrl-C to stop **");
    }

    // Stats & formatter
    let mut stats = Stats::new(addr, resolve_ms);
    let fmt: Box<dyn Formatter> = formatter::from_mode(args.output_mode);

    // Signal handling (Ctrl-C)
    let sigint = signal::ctrl_c();
    tokio::pin!(sigint);

    // Main probe loop
    loop {
        if !stats.should_continue(&args) {
            break;
        }

        let (ok, rtt) = probe_once(addr, timeout).await;
        let res = stats.feed(ok, rtt, args.jitter);
        fmt.probe(&res);

        if stats.should_break(ok, &args) {
            break;
        }

        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {},
            _ = &mut sigint => break,
        }
    }

    // Summary
    fmt.summary(&stats.summary());
    Ok(stats.exit_code())
}
