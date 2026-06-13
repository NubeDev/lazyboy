use lazyboy_store::OutboxEventRow;

use crate::event::SyncEvent;
use crate::SyncError;

/// A single outbox row mapped to what the publisher puts on the wire:
/// the Zenoh key it goes to and the serialized `SyncEvent` bytes. Pure
/// and deterministic so it is unit-testable without a live session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub key: String,
    pub payload: Vec<u8>,
}

/// The key an event publishes on. Scoped by workspace so peers can sub
/// a whole workspace with one `lazyboy/{workspace}/**` expression, then
/// narrowed by aggregate and id for selective subscriptions.
pub fn key_for(workspace: &str, aggregate: &str, aggregate_id: &str) -> String {
    format!("lazyboy/{workspace}/{aggregate}/{aggregate_id}")
}

/// Map one drained outbox row into its publication. The row's stored
/// `event_json` becomes the `SyncEvent` payload; `created_at` is the
/// LWW clock the far side compares on.
pub fn to_publication(workspace: &str, row: &OutboxEventRow) -> Result<Publication, SyncError> {
    let payload: serde_json::Value = serde_json::from_str(&row.event_json)?;
    let event = SyncEvent {
        aggregate: row.aggregate.clone(),
        aggregate_id: row.aggregate_id.clone(),
        seq: row.seq,
        occurred_at: row.created_at,
        payload,
    };
    Ok(Publication {
        key: key_for(workspace, &row.aggregate, &row.aggregate_id),
        payload: serde_json::to_vec(&event)?,
    })
}
