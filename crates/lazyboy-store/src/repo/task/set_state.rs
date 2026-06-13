use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{Task, TaskState};
use lazyboy_types::Id;

/// Advance a task's lifecycle state and stamp `updated_at`.
pub async fn set_state(
    store: &Store,
    task_id: Id<Task>,
    state: TaskState,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE tasks SET state = ?, updated_at = ? WHERE id = ?")
        .bind(state.as_str())
        .bind(clock::fmt(clock::now()))
        .bind(task_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
