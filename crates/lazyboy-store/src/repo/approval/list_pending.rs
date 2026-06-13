use crate::{ApprovalRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// The approval queue for a space: every still-pending tool request,
/// oldest first.
pub async fn list_pending(
    store: &Store,
    space_id: Id<Space>,
) -> Result<Vec<ApprovalRow>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM approvals WHERE space_id = ? AND status = 'pending' \
         ORDER BY requested_at",
    )
    .bind(space_id.to_string())
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(ApprovalRow::from_row).collect()
}
