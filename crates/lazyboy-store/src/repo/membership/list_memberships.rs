use crate::{SpaceMembershipRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// A space's memberships, ordered by id for a stable listing. Used by
/// tests and the modeled (not enforced) membership surface.
pub async fn list_memberships(
    store: &Store,
    space_id: Id<Space>,
) -> Result<Vec<SpaceMembershipRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM space_memberships WHERE space_id = ? ORDER BY id")
        .bind(space_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(SpaceMembershipRow::from_row).collect()
}
