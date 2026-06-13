//! Lazyboy domain types — the wire vocabulary shared by every crate.
//!
//! The SQLite domain model in `SCOPE.md` is the source of truth for
//! the state enums here; each lives in its own file under `domain/`.

pub mod domain;
mod id;

pub use id::Id;
