use crate::{IngressEventRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// A space's ingress audit trail, oldest first — every external event
/// that landed there, deduped at ingest, with its mapped message.
pub async fn list(store: &Store, space_id: Id<Space>) -> Result<Vec<IngressEventRow>, StoreError> {
    let rows =
        sqlx::query("SELECT * FROM ingress_events WHERE space_id = ? ORDER BY received_at, id")
            .bind(space_id.to_string())
            .fetch_all(store.pool())
            .await?;
    rows.iter().map(IngressEventRow::from_row).collect()
}
