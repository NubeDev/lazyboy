use crate::{Store, StoreError, TaskRow};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// Every task in a space, oldest first. Feeds the right-panel task list
/// of the cowork UI.
pub async fn list(store: &Store, space_id: Id<Space>) -> Result<Vec<TaskRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM tasks WHERE space_id = ? ORDER BY created_at, id")
        .bind(space_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(TaskRow::from_row).collect()
}
