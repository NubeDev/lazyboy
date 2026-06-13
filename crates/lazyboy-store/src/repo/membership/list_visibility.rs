use crate::{FeedVisibilityRow, Store, StoreError};
use lazyboy_types::domain::Integration;
use lazyboy_types::Id;

/// A feed's visibility rules, ordered by id for a stable listing.
pub async fn list_visibility(
    store: &Store,
    feed_integration_id: Id<Integration>,
) -> Result<Vec<FeedVisibilityRow>, StoreError> {
    let rows =
        sqlx::query("SELECT * FROM feed_visibility WHERE feed_integration_id = ? ORDER BY id")
            .bind(feed_integration_id.to_string())
            .fetch_all(store.pool())
            .await?;
    rows.iter().map(FeedVisibilityRow::from_row).collect()
}
