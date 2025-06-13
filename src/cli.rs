//! Argument parsing layer (clap).

use clap::{Parser, ValueEnum};

/// Global CLI arguments.
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Target in the form `<host:port>`
    pub address: String,

    /// Number of probes (`-c`)
    #[arg(short, long, default_value_t = 4)]
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
    #[arg(long, default_value_t = 2000)]
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
