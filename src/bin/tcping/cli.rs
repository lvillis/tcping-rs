//! Argument parsing layer (clap).

use clap::{ArgAction, ArgGroup, Parser, ValueEnum};

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let count: usize = value
        .parse()
        .map_err(|_| format!("'{value}' is not a valid number"))?;
    if count == 0 {
        Err("count must be >= 1".into())
    } else {
        Ok(count)
    }
}

fn parse_positive_u64(value: &str) -> Result<u64, String> {
    let timeout: u64 = value
        .parse()
        .map_err(|_| format!("'{value}' is not a valid number"))?;
    if timeout == 0 {
        Err("timeout must be >= 1".into())
    } else {
        Ok(timeout)
    }
}

/// Global CLI arguments.
#[derive(Parser, Debug)]
#[command(author, version, about)]
#[command(group(
    ArgGroup::new("timestamp_mode")
        .args(["timestamp", "unix_timestamp"])
        .multiple(false)
))]
pub(crate) struct Args {
    /// Target in the form `<host:port>`
    pub(crate) address: String,

    /// Number of probes (`-c`)
    #[arg(
        short,
        long,
        default_value_t = 4,
        value_parser = parse_positive_usize,
        help = "Total probes to send (must be >= 1)"
    )]
    pub(crate) count: usize,

    /// Keep probing until Ctrl-C (`-t`)
    #[arg(short = 't', long)]
    pub(crate) continuous: bool,

    /// Output format (`-o`)
    #[arg(
        short = 'o',
        long,
        value_enum,
        default_value_t = OutputMode::Normal,
        help = "normal | json | csv | md | color"
    )]
    pub(crate) output_mode: OutputMode,

    /// Exit after first success (`-e`)
    #[arg(short = 'e', long)]
    pub(crate) exit_on_success: bool,

    /// Show per-probe jitter (`-j`)
    #[arg(short = 'j', long)]
    pub(crate) jitter: bool,

    /// Emit timestamps with every probe and summary record
    #[arg(
        long,
        visible_alias = "date",
        value_enum,
        num_args = 0..=1,
        default_missing_value = "iso8601",
        group = "timestamp_mode",
        help = "Emit timestamps with every probe and summary record (default: iso8601)"
    )]
    pub(crate) timestamp: Option<TimestampFormat>,

    /// Shorthand for `--timestamp unix`
    #[arg(
        short = 'D',
        action = ArgAction::SetTrue,
        group = "timestamp_mode",
        help = "Emit Unix epoch timestamps with every probe and summary record"
    )]
    pub(crate) unix_timestamp: bool,

    /// Timeout per probe (ms)
    #[arg(
        long,
        default_value_t = 2000,
        value_parser = parse_positive_u64,
        help = "Per-probe timeout in milliseconds (must be >= 1)"
    )]
    pub(crate) timeout_ms: u64,
}

/// Supported output modes.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputMode {
    Normal,
    Json,
    Csv,
    Md,    // Markdown
    Color, // ANSI-colored TTY
}

/// Human-facing timestamp styles.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimestampFormat {
    Unix,
    Iso8601,
}

impl Args {
    /// Resolve the requested timestamp mode after clap parsing.
    pub(crate) fn timestamp_format(&self) -> Option<TimestampFormat> {
        if self.unix_timestamp {
            Some(TimestampFormat::Unix)
        } else {
            self.timestamp
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::net::ToSocketAddrs;

    #[test]
    fn parse_basic() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "-c", "5"]);
        assert_eq!(a.address, "127.0.0.1:80");
        assert_eq!(a.count, 5);
        assert!(!a.continuous);
        assert_eq!(a.output_mode, OutputMode::Normal);
        assert_eq!(a.timestamp_format(), None);
    }

    #[test]
    fn continuous_flag() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "-t"]);
        assert!(a.continuous);
    }

    #[test]
    fn resolve_localhost() {
        assert!("localhost:80".to_socket_addrs().is_ok());
    }

    #[test]
    fn output_mode_json() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "-o", "json"]);
        assert_eq!(a.output_mode, OutputMode::Json);
    }

    #[test]
    fn exit_on_success() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "-e"]);
        assert!(a.exit_on_success);
    }

    #[test]
    fn reject_zero_count() {
        let err = Args::try_parse_from(["tcping", "127.0.0.1:80", "-c", "0"]).unwrap_err();
        assert!(err.to_string().contains(">= 1"));
    }

    #[test]
    fn reject_zero_timeout() {
        let err =
            Args::try_parse_from(["tcping", "127.0.0.1:80", "--timeout-ms", "0"]).unwrap_err();
        assert!(err.to_string().contains(">= 1"));
    }

    #[test]
    fn timestamp_defaults_to_iso8601_when_enabled_without_value() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "--timestamp"]);
        assert_eq!(a.timestamp_format(), Some(TimestampFormat::Iso8601));
    }

    #[test]
    fn date_alias_enables_iso8601_timestamps() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "--date"]);
        assert_eq!(a.timestamp_format(), Some(TimestampFormat::Iso8601));
    }

    #[test]
    fn uppercase_d_enables_unix_timestamps() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "-D"]);
        assert_eq!(a.timestamp_format(), Some(TimestampFormat::Unix));
    }

    #[test]
    fn timestamp_accepts_explicit_unix_value() {
        let a = Args::parse_from(["tcping", "127.0.0.1:80", "--timestamp", "unix"]);
        assert_eq!(a.timestamp_format(), Some(TimestampFormat::Unix));
    }
}
