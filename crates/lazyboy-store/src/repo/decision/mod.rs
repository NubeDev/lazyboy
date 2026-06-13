//! Decisions: the space's durable record of what was settled (SCOPE.md
//! build step 4). `record` writes one anchored to its timeline message;
//! `list` reads them back for the space.

mod list;
mod record;

pub use list::list;
pub use record::{record, NewDecision};
