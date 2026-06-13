use crate::{Store, StoreError, WorkflowRow};
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

/// A workspace's workflows, ordered by creation for a stable listing.
pub async fn list(
    store: &Store,
    workspace_id: Id<Workspace>,
) -> Result<Vec<WorkflowRow>, StoreError> {
    let rows =
        sqlx::query("SELECT * FROM workflows WHERE workspace_id = ? ORDER BY created_at, id")
            .bind(workspace_id.to_string())
            .fetch_all(store.pool())
            .await?;
    rows.iter().map(WorkflowRow::from_row).collect()
}
