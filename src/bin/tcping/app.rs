//! CLI application adapter built on top of the library session API.

use crate::{
    cli::Args,
    formatter::{self, Formatter},
};
use std::{ops::ControlFlow, time::Duration};
use tcping::{PingEvent, PingOptions, Result, Target, run_with_handler_until};
use tokio::signal;

/// Create a two-thread Tokio runtime and block on the async CLI runner.
pub(crate) fn run(args: Args) -> Result<i32> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(run_async(args))
}

/// Run one CLI tcping session.
pub(crate) async fn run_async(args: Args) -> Result<i32> {
    let timestamp_format = args.timestamp_format();
    let options = options_from_args(&args)?;
    let mut fmt = formatter::from_mode(args.output_mode, timestamp_format);

    let summary = run_with_handler_until(options, signal::ctrl_c(), |event| {
        emit_event(&mut *fmt, event);
        ControlFlow::Continue(())
    })
    .await?;

    Ok(summary.exit_code())
}

fn emit_event(fmt: &mut dyn Formatter, event: PingEvent) {
    match event {
        PingEvent::Resolved(target) => fmt.resolved(&target),
        PingEvent::Probe(result) => fmt.probe(&result),
        PingEvent::Summary(summary) => fmt.summary(&summary),
        _ => {}
    }
}

fn options_from_args(args: &Args) -> Result<PingOptions> {
    let target = Target::parse(&args.address)?;
    let options = PingOptions::new(target)
        .with_count(args.count)?
        .with_timeout(Duration::from_millis(args.timeout_ms))
        .exit_on_success(args.exit_on_success)
        .jitter(args.jitter)
        .timestamps(args.timestamp_format().is_some());

    Ok(if args.continuous {
        options.continuous()
    } else {
        options
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputMode;
    use clap::Parser;
    use tcping::ProbeCount;

    #[test]
    fn args_map_to_library_options() {
        let args = Args::parse_from([
            "tcping",
            "127.0.0.1:80",
            "-c",
            "3",
            "--timeout-ms",
            "250",
            "-e",
            "-j",
            "--timestamp",
        ]);

        let options = options_from_args(&args).unwrap();
        assert_eq!(options.target().host(), "127.0.0.1");
        assert_eq!(options.probes(), ProbeCount::fixed(3).unwrap());
        assert_eq!(options.timeout(), Duration::from_millis(250));
        assert!(options.exits_on_success());
        assert!(options.includes_jitter());
        assert!(options.includes_timestamps());
    }

    #[test]
    fn continuous_flag_maps_to_continuous_probe_count() {
        let args = Args::parse_from(["tcping", "127.0.0.1:80", "-t"]);
        let options = options_from_args(&args).unwrap();
        assert_eq!(options.probes(), ProbeCount::Continuous);
    }

    #[test]
    fn output_mode_is_not_part_of_library_options() {
        let args = Args::parse_from(["tcping", "127.0.0.1:80", "-o", "json"]);
        let options = options_from_args(&args).unwrap();
        assert_eq!(args.output_mode, OutputMode::Json);
        assert_eq!(options.target().port(), 80);
    }
}
