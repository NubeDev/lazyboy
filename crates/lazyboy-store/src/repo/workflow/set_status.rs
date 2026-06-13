use crate::{Store, StoreError};
use lazyboy_types::domain::{Workflow, WorkflowStatus};
use lazyboy_types::Id;

/// Arm or disarm a workflow's trigger. `enabled` is what SCOPE.md calls
/// an automation (the live, triggerable form); `disabled` keeps the
/// saved workflow inert. This is the only mutation the trigger state
/// needs.
pub async fn set_status(
    store: &Store,
    workflow_id: Id<Workflow>,
    status: WorkflowStatus,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE workflows SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(workflow_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
