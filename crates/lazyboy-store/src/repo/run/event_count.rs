use sqlx::Row;

use crate::{Store, StoreError};
use lazyboy_types::domain::AgentRun;
use lazyboy_types::Id;

/// How many events have been imported for a run. Used by the
/// crash-resume reconcile to seed its seq counter past replayed
/// history so re-import is a no-op rather than a collision.
pub async fn event_count(store: &Store, run_id: Id<AgentRun>) -> Result<i64, StoreError> {
    let row = sqlx::query("SELECT COUNT(*) AS n FROM agent_run_events WHERE agent_run_id = ?")
        .bind(run_id.to_string())
        .fetch_one(store.pool())
        .await?;
    Ok(row.try_get("n")?)
}
