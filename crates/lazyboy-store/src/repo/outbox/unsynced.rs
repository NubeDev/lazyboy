use crate::row::OutboxEventRow;
use crate::{Store, StoreError};

/// The replication queue: every appended event not yet shipped, oldest
/// first. Ordered by `created_at` then `seq` so a publisher drains in
/// the order changes happened, and ties within a millisecond fall back
/// to the per-aggregate monotonic seq.
pub async fn unsynced(store: &Store) -> Result<Vec<OutboxEventRow>, StoreError> {
    let rows = sqlx::query(
        "SELECT id, aggregate, aggregate_id, event_json, seq, created_at, synced_at \
         FROM outbox_events WHERE synced_at IS NULL ORDER BY created_at, seq",
    )
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(OutboxEventRow::from_row).collect()
}
