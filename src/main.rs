//! Binary bootstrap — keep it minimal.

mod cli;
mod engine;
mod error;
mod formatter;
mod probe;
mod stats;

use clap::Parser;
use cli::Args;
use error::Result;

fn main() -> Result<()> {
    // Parse CLI and delegate everything to `engine::run`.
    let args = Args::parse();
    let exit_code = engine::run(args)?;
    std::process::exit(exit_code);
}
