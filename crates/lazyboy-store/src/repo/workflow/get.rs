use crate::{Store, StoreError, WorkflowRow};
use lazyboy_types::domain::Workflow;
use lazyboy_types::Id;

pub async fn get(store: &Store, workflow_id: Id<Workflow>) -> Result<WorkflowRow, StoreError> {
    let row = sqlx::query("SELECT * FROM workflows WHERE id = ?")
        .bind(workflow_id.to_string())
        .fetch_optional(store.pool())
        .await?
        .ok_or_else(|| StoreError::NotFound(format!("workflow {workflow_id}")))?;
    WorkflowRow::from_row(&row)
}
