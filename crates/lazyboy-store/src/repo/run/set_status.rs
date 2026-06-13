use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, RunStatus};
use lazyboy_types::Id;

/// Move a run to a new status. Terminal statuses stamp `ended_at` so
/// the reconcile can tell a crashed-mid-run from a finished one.
pub async fn set_status(
    store: &Store,
    run_id: Id<AgentRun>,
    status: RunStatus,
) -> Result<(), StoreError> {
    let ended = if status.is_live() {
        None
    } else {
        Some(clock::fmt(clock::now()))
    };
    sqlx::query("UPDATE agent_runs SET status = ?, ended_at = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(ended)
        .bind(run_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
