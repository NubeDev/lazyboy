//! SQLite store — the local source of truth (SCOPE.md R1).
//!
//! `Store` owns the pool. Row types live one-per-file under `row/`;
//! query verbs live one-per-file under `repo/`. The UI never reaches
//! past this crate to anything authoritative.

mod error;
mod open;
pub mod repo;
mod row;

pub use error::StoreError;
pub use open::Store;
pub use row::{
    ApprovalRow, ArtifactRow, CalendarEventRow, DecisionRow, FeedVisibilityRow, GroupRow,
    IngressEventRow, IntegrationRow, MessageRow, OutboxEventRow, ReminderRow, RunRow,
    SpaceMembershipRow, SpaceRow, TaskRow, WorkflowRow, WorkflowRunRow,
};
