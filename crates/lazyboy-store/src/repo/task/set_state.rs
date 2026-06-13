use crate::repo::{clock, outbox};
use crate::{Store, StoreError};
use lazyboy_types::domain::{Task, TaskState};
use lazyboy_types::Id;

/// Advance a task's lifecycle state and stamp `updated_at`.
pub async fn set_state(
    store: &Store,
    task_id: Id<Task>,
    state: TaskState,
) -> Result<(), StoreError> {
    let now = clock::now();
    sqlx::query("UPDATE tasks SET state = ?, updated_at = ? WHERE id = ?")
        .bind(state.as_str())
        .bind(clock::fmt(now))
        .bind(task_id.to_string())
        .execute(store.pool())
        .await?;

    // A task row is mutable, so a peer resolves concurrent edits by
    // last-writer-wins on `updated_at` (DOCS/ZENOH.md, Open Question 1);
    // the field is shipped here so the far side can compare without
    // re-reading.
    let event = serde_json::json!({
        "op": "task.set_state",
        "id": task_id.to_string(),
        "state": state.as_str(),
        "updated_at": clock::fmt(now),
    });
    outbox::record(store, "task", &task_id.to_string(), &event.to_string()).await?;

    Ok(())
}
