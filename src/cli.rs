//! Argument parsing layer (clap).

use clap::{Parser, ValueEnum};

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
pub struct Args {
    /// Target in the form `<host:port>`
    pub address: String,

    /// Number of probes (`-c`)
    #[arg(
        short,
        long,
        default_value_t = 4,
        value_parser = parse_positive_usize,
        help = "Total probes to send (must be >= 1)"
    )]
    pub count: usize,

    /// Keep probing until Ctrl-C (`-t`)
    #[arg(short = 't', long)]
    pub continuous: bool,

    /// Output format (`-o`)
    #[arg(
        short = 'o',
        long,
        value_enum,
        default_value_t = OutputMode::Normal,
        help = "normal | json | csv | md | color"
    )]
    pub output_mode: OutputMode,

    /// Exit after first success (`-e`)
    #[arg(short = 'e', long)]
    pub exit_on_success: bool,

    /// Show per-probe jitter (`-j`)
    #[arg(short = 'j', long)]
    pub jitter: bool,

    /// Timeout per probe (ms)
    #[arg(
        long,
        default_value_t = 2000,
        value_parser = parse_positive_u64,
        help = "Per-probe timeout in milliseconds (must be >= 1)"
    )]
    pub timeout_ms: u64,
}

/// Supported output modes.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    Normal,
    Json,
    Csv,
    Md,    // Markdown
    Color, // ANSI-colored TTY
}
