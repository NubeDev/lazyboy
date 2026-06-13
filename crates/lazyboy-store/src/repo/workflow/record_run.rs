use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, Space, Workflow, WorkflowRun};
use lazyboy_types::Id;

/// Record that a workflow fired, linking it to the agent run it created
/// (SCOPE.md). Written `started_at`-stamped with no end; the firing path
/// stamps `ended_at` via `finish_run` once the drive returns.
pub async fn record_run(
    store: &Store,
    workflow_id: Id<Workflow>,
    space_id: Id<Space>,
    agent_run_id: Id<AgentRun>,
) -> Result<Id<WorkflowRun>, StoreError> {
    let id = Id::<WorkflowRun>::new();
    sqlx::query(
        "INSERT INTO workflow_runs (id, workflow_id, space_id, agent_run_id, status, \
         started_at, ended_at) VALUES (?, ?, ?, ?, 'running', ?, NULL)",
    )
    .bind(id.to_string())
    .bind(workflow_id.to_string())
    .bind(space_id.to_string())
    .bind(agent_run_id.to_string())
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(id)
}

/// Stamp a workflow run's terminal status and end time once its drive
/// settles (ended, or parked awaiting an approval).
pub async fn finish_run(
    store: &Store,
    workflow_run_id: Id<WorkflowRun>,
    status: &str,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE workflow_runs SET status = ?, ended_at = ? WHERE id = ?")
        .bind(status)
        .bind(clock::fmt(clock::now()))
        .bind(workflow_run_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
