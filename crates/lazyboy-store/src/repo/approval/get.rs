use crate::{ApprovalRow, Store, StoreError};
use lazyboy_types::domain::Approval;
use lazyboy_types::Id;

pub async fn get(store: &Store, approval_id: Id<Approval>) -> Result<ApprovalRow, StoreError> {
    let row = sqlx::query("SELECT * FROM approvals WHERE id = ?")
        .bind(approval_id.to_string())
        .fetch_optional(store.pool())
        .await?
        .ok_or_else(|| StoreError::NotFound(format!("approval {approval_id}")))?;
    ApprovalRow::from_row(&row)
}
