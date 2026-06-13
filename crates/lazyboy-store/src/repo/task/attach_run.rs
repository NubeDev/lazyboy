use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, Task};
use lazyboy_types::Id;

/// Record the agent run now driving this task.
pub async fn attach_run(
    store: &Store,
    task_id: Id<Task>,
    run_id: Id<AgentRun>,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE tasks SET agent_run_id = ?, updated_at = ? WHERE id = ?")
        .bind(run_id.to_string())
        .bind(clock::fmt(clock::now()))
        .bind(task_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
