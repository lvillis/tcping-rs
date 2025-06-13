//! Orchestrator: resolve target → fixed-interval probes → formatted output.
//!
//! Features
//! --------
//! • Multi-thread Tokio runtime (I/O thread + timer thread).  
//! • Windows-only tweaks in `main.rs` boost timer resolution and thread
//!   priority (see `src/main.rs`).  
//! • `tokio::time::interval` with the *first* tick pre-consumed so probes
//!   print exactly every second—no first-loop double print.  
//! • Domain targets show a “Resolved …” banner with DNS server & timing.
//! • Sequence number travels via `PingResult.seq`; Normal/Color formatters
//!   add a one-shot note after the very first probe.

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
use tokio::{signal, time};

/// Best-effort reader for the first DNS server listed in `/etc/resolv.conf`
/// (Unix only).  Returns `None` on Windows or failure.
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
    let (host, _port) = args
        .address
        .split_once(':')
        .ok_or_else(|| TcpingError::Other(anyhow::anyhow!("target must be host:port")))?;
    let is_ip_literal = host.parse::<IpAddr>().is_ok();

    /* ---------- resolution ---------- */
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

    /* ---------- blank line for IP literal ---------- */
    if is_ip_literal && matches!(args.output_mode, OutputMode::Normal) {
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
