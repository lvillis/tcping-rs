//! Common error wrapper.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TcpingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Tokio join error: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Handy alias.
pub type Result<T> = std::result::Result<T, TcpingError>;
