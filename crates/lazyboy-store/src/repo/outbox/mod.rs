//! The replication outbox (SCOPE.md "Zenoh sync fabric"). Every state
//! change is appended here as an event; the sync layer drains it and
//! ships each event to peers. This crate holds no networking — the
//! outbox is the stable local boundary that must be solid before any
//! Zenoh code turns on (the "no Zenoh until the local event model is
//! stable" gate). Verbs: `append` enqueues, `unsynced` is the queue,
//! `mark_synced` acknowledges. `record` is the convenience wrapper a
//! mutation verb calls with an already-serialized event payload.

mod append;
mod mark_synced;
mod unsynced;

pub use append::{append, NewOutboxEvent};
pub use mark_synced::mark_synced;
pub use unsynced::unsynced;

use crate::{Store, StoreError};

/// Record a state change for an aggregate row, given its already-built
/// JSON payload. Thin wrapper over `append` so a mutation verb wires
/// the outbox in one line without restating the column set. Kept
/// payload-agnostic on purpose: the store does not own the event
/// schema, the sync crate and callers do.
pub async fn record(
    store: &Store,
    aggregate: &str,
    aggregate_id: &str,
    event_json: &str,
) -> Result<(), StoreError> {
    append(
        store,
        NewOutboxEvent {
            aggregate,
            aggregate_id,
            event_json,
        },
    )
    .await?;
    Ok(())
}
