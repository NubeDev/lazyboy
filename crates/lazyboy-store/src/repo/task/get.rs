use crate::{Store, StoreError, TaskRow};
use lazyboy_types::domain::Task;
use lazyboy_types::Id;

pub async fn get(store: &Store, task_id: Id<Task>) -> Result<TaskRow, StoreError> {
    let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
        .bind(task_id.to_string())
        .fetch_optional(store.pool())
        .await?
        .ok_or_else(|| StoreError::NotFound(format!("task {task_id}")))?;
    TaskRow::from_row(&row)
}
