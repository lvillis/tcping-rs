//! Library facade - re-export internal modules so integration
//! tests or external code can use `tcping::...`.

pub mod cli;
pub mod engine;
pub mod error;
pub mod formatter;
pub mod probe;
pub mod stats;
