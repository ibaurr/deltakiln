//! Shared foundation for deltakiln: configuration, errors and filesystem helpers.
//!
//! The dependency graph, hashing and cache arrive in M1; M0 only needs the
//! pieces the naive full build relies on.

pub mod config;
pub mod error;
pub mod fsutil;

pub use config::Config;
pub use error::{Error, Result};
