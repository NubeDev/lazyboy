use crate::{SpaceRow, Store, StoreError};

/// Every space in the node, newest workspace ordering by creation. The
/// left rail of the cowork UI is built from this (SCOPE.md "left rail of
/// spaces").
pub async fn list(store: &Store) -> Result<Vec<SpaceRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM spaces ORDER BY created_at, id")
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(SpaceRow::from_row).collect()
}
