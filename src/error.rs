//! Common error wrapper.

use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum TcpingError {
    #[error("invalid target: {0}")]
    InvalidTarget(String),

    #[error("invalid options: {0}")]
    InvalidOptions(String),

    #[error("target did not resolve to any socket address")]
    NoAddress,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tokio join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// Handy alias.
pub type Result<T> = std::result::Result<T, TcpingError>;
