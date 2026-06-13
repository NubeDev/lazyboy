use crate::{ArtifactRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// Every artifact a space has produced, oldest first.
pub async fn list(store: &Store, space_id: Id<Space>) -> Result<Vec<ArtifactRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM artifacts WHERE space_id = ? ORDER BY created_at, id")
        .bind(space_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(ArtifactRow::from_row).collect()
}
