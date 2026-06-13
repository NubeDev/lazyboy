use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, RunStatus, Space, Task};
use lazyboy_types::Id;

/// Queue a new run for a task. The Goose session id is unknown until
/// the bridge opens the session, so it starts NULL and is filled by
/// `set_session`.
pub async fn create(
    store: &Store,
    space_id: Id<Space>,
    task_id: Id<Task>,
) -> Result<Id<AgentRun>, StoreError> {
    let id = Id::<AgentRun>::new();
    sqlx::query(
        "INSERT INTO agent_runs (id, space_id, task_id, goose_session_id, status, \
         started_at, ended_at) VALUES (?, ?, ?, NULL, ?, ?, NULL)",
    )
    .bind(id.to_string())
    .bind(space_id.to_string())
    .bind(task_id.to_string())
    .bind(RunStatus::Queued.as_str())
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(id)
}
