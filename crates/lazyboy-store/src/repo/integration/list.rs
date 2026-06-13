use crate::{IntegrationRow, Store, StoreError};
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

/// A workspace's integrations, ordered by id for a stable listing.
pub async fn list(
    store: &Store,
    workspace_id: Id<Workspace>,
) -> Result<Vec<IntegrationRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM integrations WHERE workspace_id = ? ORDER BY id")
        .bind(workspace_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(IntegrationRow::from_row).collect()
}
