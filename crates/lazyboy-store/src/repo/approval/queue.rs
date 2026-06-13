use crate::{ApprovalRow, Store, StoreError};
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

/// The workspace-wide approval queue (SCOPE.md build step 2): every
/// still-pending tool request across all of a workspace's spaces,
/// oldest first. `list_pending` is the per-space slice of this; the
/// queue view is what a single human watching the whole workspace
/// works through (single-tenant trust boundary, R4).
pub async fn queue(
    store: &Store,
    workspace_id: Id<Workspace>,
) -> Result<Vec<ApprovalRow>, StoreError> {
    let rows = sqlx::query(
        "SELECT a.* FROM approvals a JOIN spaces s ON a.space_id = s.id \
         WHERE s.workspace_id = ? AND a.status = 'pending' \
         ORDER BY a.requested_at",
    )
    .bind(workspace_id.to_string())
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(ApprovalRow::from_row).collect()
}
