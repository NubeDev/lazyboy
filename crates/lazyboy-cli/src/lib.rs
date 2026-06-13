//! The step-1 shell as a library, so integration tests can drive the
//! same `commands` the `lazyboy` binary calls (against a fake goose)
//! rather than shelling out. The binary (`main.rs`) is a thin arg-parsing
//! wrapper over these.

pub mod commands;
pub mod config;
mod error;

pub use config::Config;
pub use error::CliError;
