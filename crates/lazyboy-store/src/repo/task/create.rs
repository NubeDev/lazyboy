use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{Message, Space, Task, TaskState};
use lazyboy_types::Id;

/// Open a task in a space, optionally noting the message it was
/// distilled from ("talk becomes tasks"). Starts in `Open`.
pub async fn create(
    store: &Store,
    space_id: Id<Space>,
    title: &str,
    from_message: Option<Id<Message>>,
) -> Result<Id<Task>, StoreError> {
    let id = Id::<Task>::new();
    let now = clock::fmt(clock::now());
    sqlx::query(
        "INSERT INTO tasks (id, space_id, title, state, created_from_message_id, \
         agent_run_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(id.to_string())
    .bind(space_id.to_string())
    .bind(title)
    .bind(TaskState::Open.as_str())
    .bind(from_message.map(|m| m.to_string()))
    .bind(&now)
    .bind(&now)
    .execute(store.pool())
    .await?;
    Ok(id)
}
