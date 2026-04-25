//! Session runner APIs for library callers.

use crate::{
    error::{Result, TcpingError},
    probe::probe_once,
    stats::{PingResult, Stats, Summary},
    target::{ResolvedTarget, Target, resolve_target},
    timestamp::RecordTimestamp,
};
use std::{
    future::{Future, pending},
    num::NonZeroUsize,
    ops::ControlFlow,
    time::Duration,
};
use tokio::time;

/// Number of probes to run in a session.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProbeCount {
    Fixed(NonZeroUsize),
    Continuous,
}

impl ProbeCount {
    /// Build a fixed probe count.
    pub fn fixed(count: usize) -> Result<Self> {
        let count = NonZeroUsize::new(count)
            .ok_or_else(|| TcpingError::InvalidOptions("probe count must be >= 1".into()))?;
        Ok(Self::Fixed(count))
    }

    fn should_continue(self, attempts: usize) -> bool {
        match self {
            Self::Fixed(count) => attempts < count.get(),
            Self::Continuous => true,
        }
    }
}

/// Probe session configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct PingOptions {
    target: Target,
    probes: ProbeCount,
    interval: Duration,
    timeout: Duration,
    exit_on_success: bool,
    jitter: bool,
    timestamps: bool,
}

impl PingOptions {
    /// Create options with CLI-compatible defaults: 4 probes, 1s interval, 2s timeout.
    pub fn new(target: Target) -> Self {
        Self {
            target,
            probes: ProbeCount::Fixed(NonZeroUsize::new(4).expect("4 is non-zero")),
            interval: Duration::from_secs(1),
            timeout: Duration::from_millis(2_000),
            exit_on_success: false,
            jitter: false,
            timestamps: false,
        }
    }

    pub fn with_count(mut self, count: usize) -> Result<Self> {
        self.probes = ProbeCount::fixed(count)?;
        Ok(self)
    }

    pub fn continuous(mut self) -> Self {
        self.probes = ProbeCount::Continuous;
        self
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn exit_on_success(mut self, exit_on_success: bool) -> Self {
        self.exit_on_success = exit_on_success;
        self
    }

    pub fn jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    pub fn timestamps(mut self, timestamps: bool) -> Self {
        self.timestamps = timestamps;
        self
    }

    pub fn target(&self) -> &Target {
        &self.target
    }

    pub fn probes(&self) -> ProbeCount {
        self.probes
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn exits_on_success(&self) -> bool {
        self.exit_on_success
    }

    pub fn includes_jitter(&self) -> bool {
        self.jitter
    }

    pub fn includes_timestamps(&self) -> bool {
        self.timestamps
    }

    fn validate(&self) -> Result<()> {
        if self.interval.is_zero() {
            return Err(TcpingError::InvalidOptions(
                "probe interval must be greater than zero".into(),
            ));
        }

        if self.timeout.is_zero() {
            return Err(TcpingError::InvalidOptions(
                "probe timeout must be greater than zero".into(),
            ));
        }

        Ok(())
    }
}

/// Event emitted during a probing session.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum PingEvent {
    Resolved(ResolvedTarget),
    Probe(PingResult),
    Summary(Summary),
}

/// Complete result for a finite probing session.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PingSession {
    pub resolved: ResolvedTarget,
    pub probes: Vec<PingResult>,
    pub summary: Summary,
}

/// Run a finite session and collect all emitted data.
///
/// Continuous sessions are rejected because collecting them would grow memory
/// without a natural bound. Use [`run_with_handler_async`] for continuous work.
pub async fn run_collect_async(options: PingOptions) -> Result<PingSession> {
    if matches!(options.probes, ProbeCount::Continuous) {
        return Err(TcpingError::InvalidOptions(
            "run_collect_async requires a fixed probe count".into(),
        ));
    }

    let mut resolved = None;
    let mut probes = Vec::new();
    let mut final_summary = None;

    let summary = run_with_handler_async(options, |event| {
        match event {
            PingEvent::Resolved(value) => resolved = Some(value),
            PingEvent::Probe(value) => probes.push(value),
            PingEvent::Summary(value) => final_summary = Some(value),
        }
        ControlFlow::Continue(())
    })
    .await?;

    Ok(PingSession {
        resolved: resolved.ok_or(TcpingError::NoAddress)?,
        probes,
        summary: final_summary.unwrap_or(summary),
    })
}

/// Blocking wrapper around [`run_collect_async`].
pub fn run_collect(options: PingOptions) -> Result<PingSession> {
    runtime()?.block_on(run_collect_async(options))
}

/// Run a session and push owned events to a caller-supplied handler.
///
/// Returning [`ControlFlow::Break`] from the handler stops the session after the
/// current event and still returns a final summary.
pub async fn run_with_handler_async<F>(options: PingOptions, handler: F) -> Result<Summary>
where
    F: FnMut(PingEvent) -> ControlFlow<()>,
{
    run_with_handler_until(options, pending::<()>(), handler).await
}

/// Blocking wrapper around [`run_with_handler_async`].
pub fn run_with_handler<F>(options: PingOptions, handler: F) -> Result<Summary>
where
    F: FnMut(PingEvent) -> ControlFlow<()>,
{
    runtime()?.block_on(run_with_handler_async(options, handler))
}

/// Run a session until the configured probe count is reached, the handler
/// breaks, `exit_on_success` trips, or the cancellation future completes.
pub async fn run_with_handler_until<F, C>(
    options: PingOptions,
    cancel: C,
    mut handler: F,
) -> Result<Summary>
where
    F: FnMut(PingEvent) -> ControlFlow<()>,
    C: Future,
{
    options.validate()?;

    let resolved = resolve_target(&options.target).await?;
    let mut stats = Stats::new(resolved.addr, resolved.resolve_time_ms, options.timestamps);

    if handler(PingEvent::Resolved(resolved.clone())).is_break() {
        return finish(&mut handler, &stats, options.timestamps);
    }

    let mut ticker = time::interval(options.interval);
    ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    ticker.tick().await;

    tokio::pin!(cancel);
    let mut first = true;

    loop {
        if first {
            first = false;
        } else {
            tokio::select! {
                _ = ticker.tick() => {},
                _ = &mut cancel => break,
            }
        }

        let (ok, rtt) = tokio::select! {
            result = probe_once(resolved.addr, options.timeout) => result,
            _ = &mut cancel => break,
        };

        let timestamp = options.timestamps.then(RecordTimestamp::now);
        let probe = stats.feed(ok, rtt, options.jitter, timestamp);
        if handler(PingEvent::Probe(probe)).is_break() {
            break;
        }

        if (options.exit_on_success && ok) || !options.probes.should_continue(stats.attempts()) {
            break;
        }
    }

    finish(&mut handler, &stats, options.timestamps)
}

fn finish<F>(handler: &mut F, stats: &Stats, timestamps: bool) -> Result<Summary>
where
    F: FnMut(PingEvent) -> ControlFlow<()>,
{
    let timestamp = timestamps.then(RecordTimestamp::now);
    let summary = stats.summary(timestamp);
    let _ = handler(PingEvent::Summary(summary.clone()));
    Ok(summary)
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_probe_count() {
        assert!(ProbeCount::fixed(0).is_err());
    }

    #[tokio::test]
    async fn collect_rejects_continuous_sessions() {
        let target = Target::parse("127.0.0.1:80").unwrap();
        let options = PingOptions::new(target).continuous();
        assert!(run_collect_async(options).await.is_err());
    }

    #[tokio::test]
    async fn handler_can_stop_after_resolution() {
        let target = Target::parse("127.0.0.1:80").unwrap();
        let options = PingOptions::new(target).with_count(3).unwrap();

        let summary = run_with_handler_async(options, |event| match event {
            PingEvent::Resolved(_) => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        })
        .await
        .unwrap();

        assert_eq!(summary.total_attempts, 0);
    }
}
