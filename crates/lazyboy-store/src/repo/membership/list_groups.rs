use crate::{GroupRow, Store, StoreError};
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

/// A workspace's groups, ordered by id for a stable listing.
pub async fn list_groups(
    store: &Store,
    workspace_id: Id<Workspace>,
) -> Result<Vec<GroupRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM groups WHERE workspace_id = ? ORDER BY id")
        .bind(workspace_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(GroupRow::from_row).collect()
}
