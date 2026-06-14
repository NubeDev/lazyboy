//! Provider management for the supervised goose: the curated provider
//! catalog the settings UI lists, the Lazyboy-owned config/secrets store
//! that persists the selection, and the supervisor that launches
//! `goose serve` with it. Grouped here because all three exist only to
//! let an operator pick and apply a provider from the UI.

pub mod catalog;
pub mod config;
pub mod supervisor;

pub use catalog::{ProviderSpec, PROVIDERS};
pub use config::{GooseConfigStore, Selection};
pub use supervisor::GooseSupervisor;
