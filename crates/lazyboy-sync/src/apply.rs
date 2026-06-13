//! Deciding what an inbound event does to the local store. The decision
//! is split from the execution so it is unit-testable without a session:
//! `decide` is pure, `apply` (feature `zenoh`) runs the result.

use crate::event::SyncEvent;
use crate::merge::{incoming_wins, MergeKey};

/// What an inbound event resolves to locally. Append-only aggregates
/// always insert (idempotent on the id); mutable aggregates either win
/// the LWW comparison and overwrite, or lose and are dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyAction {
    /// Insert-if-absent: union merge for append-only aggregates.
    InsertIfAbsent,
    /// Overwrite the local row: this event won LWW.
    Overwrite,
    /// Drop: an older or duplicate mutable-row event lost LWW.
    Skip,
}

/// Decide the action for an inbound event. `current` is the LWW key of
/// the local row when one exists and the aggregate is mutable; `None`
/// means either no local row yet or an append-only aggregate.
pub fn decide(event: &SyncEvent, current: Option<MergeKey>) -> ApplyAction {
    if event.is_append_only() {
        return ApplyAction::InsertIfAbsent;
    }
    match current {
        None => ApplyAction::Overwrite,
        Some(current) => {
            let incoming = MergeKey {
                occurred_at: event.occurred_at,
                seq: event.seq,
            };
            if incoming_wins(current, incoming) {
                ApplyAction::Overwrite
            } else {
                ApplyAction::Skip
            }
        }
    }
}

#[cfg(feature = "zenoh")]
pub async fn apply(
    store: &lazyboy_store::Store,
    event: &SyncEvent,
) -> Result<(), crate::SyncError> {
    use crate::inbound;

    // Mutable aggregates need the local row's current LWW key to decide;
    // append-only aggregates ignore it.
    let current = if event.is_append_only() {
        None
    } else {
        inbound::current_key(store, event).await?
    };
    match decide(event, current) {
        ApplyAction::Skip => Ok(()),
        ApplyAction::InsertIfAbsent => inbound::insert_if_absent(store, event).await,
        ApplyAction::Overwrite => inbound::overwrite(store, event).await,
    }
}
