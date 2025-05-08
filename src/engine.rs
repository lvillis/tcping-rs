//! High-level orchestrator: resolve → probe loop → output.

use crate::{
    cli::{Args, OutputMode},
    error::{Result, TcpingError},
    formatter::{self, Formatter},
    probe::probe_once,
    stats::Stats,
};
use std::{net::ToSocketAddrs, time::Instant};
use tokio::{
    signal,
    time::{sleep, Duration},
};

pub fn run(args: Args) -> Result<i32> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(run_async(args))
}

async fn run_async(args: Args) -> Result<i32> {
    /* address resolution */
    let t0 = Instant::now();
    let addr = args
        .address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| TcpingError::Other(anyhow::anyhow!("invalid target")))?;
    if addr.port() == 0 {
        return Err(TcpingError::Other(anyhow::anyhow!("port cannot be 0")));
    }
    let resolve_ms = t0.elapsed().as_secs_f64() * 1_000.0;
    let timeout = Duration::from_millis(args.timeout_ms);

    if args.continuous && matches!(args.output_mode, OutputMode::Normal) {
        println!("\n** Probing continuously — Ctrl-C to stop **");
    }
    if matches!(args.output_mode, OutputMode::Normal) {
        println!("\nResolved address in {:.4} ms", resolve_ms);
    }

    /* stats + formatter */
    let mut stats = Stats::new(addr, resolve_ms);
    let fmt: Box<dyn Formatter> = formatter::from_mode(args.output_mode);

    /* Ctrl-C future */
    let sigint = signal::ctrl_c();
    tokio::pin!(sigint);

    /* main loop */
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

    /* summary & exit code */
    fmt.summary(&stats.summary());
    Ok(stats.exit_code())
}
