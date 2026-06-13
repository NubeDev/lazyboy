//! Lazyboy core orchestration over one space (SCOPE.md build order
//! step 1). The `Engine` turns a prompt into an agent run, drives
//! Goose through the bridge until an approval gate or end of turn,
//! applies human decisions, and reconciles in-flight approvals after
//! a crash.
//!
//! The engine is generic over `GooseClient`, so the same logic runs
//! against the live transport or `FakeGoose` with no code change.

mod cancel_run;
mod drive;
mod engine;
mod error;
mod reconcile;
mod resolve_approval;
mod retry_run;
mod run_workflow;
mod start_run;
mod workflow_agent;

pub use engine::Engine;
pub use error::CoreError;
pub use reconcile::Reconciled;
pub use start_run::{RunOutcome, StartedRun};
pub use workflow_agent::FeedEvent;
