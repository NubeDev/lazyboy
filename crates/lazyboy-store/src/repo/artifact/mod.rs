//! Artifacts produced by a run (SCOPE.md "SQLite domain model"). The
//! bridge imports them from tool results; `create` writes the row and
//! `list` feeds the space's artifact view.

mod create;
mod list;

pub use create::{create, NewArtifact};
pub use list::list;
