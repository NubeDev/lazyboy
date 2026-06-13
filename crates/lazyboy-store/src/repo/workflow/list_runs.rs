use crate::{Store, StoreError, WorkflowRunRow};
use lazyboy_types::domain::Workflow;
use lazyboy_types::Id;

/// A workflow's firings, ordered by start for a stable listing.
pub async fn list_runs(
    store: &Store,
    workflow_id: Id<Workflow>,
) -> Result<Vec<WorkflowRunRow>, StoreError> {
    let rows =
        sqlx::query("SELECT * FROM workflow_runs WHERE workflow_id = ? ORDER BY started_at, id")
            .bind(workflow_id.to_string())
            .fetch_all(store.pool())
            .await?;
    rows.iter().map(WorkflowRunRow::from_row).collect()
}
