use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::OutboxEvent;
use lazyboy_types::Id;
use time::OffsetDateTime;

/// Stamp an event as shipped, removing it from the `unsynced` queue.
/// Called once a publisher has confirmed the event left this node.
pub async fn mark_synced(
    store: &Store,
    id: Id<OutboxEvent>,
    ts: OffsetDateTime,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE outbox_events SET synced_at = ? WHERE id = ?")
        .bind(clock::fmt(ts))
        .bind(id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
