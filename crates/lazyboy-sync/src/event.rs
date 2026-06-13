use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// One replicated change, as it travels over the fabric. This is the
/// wire shape the store's outbox payloads decode into and the shape a
/// subscriber applies. It mirrors the `outbox_events` columns a peer
/// needs to order and merge: the per-aggregate `seq` and the change's
/// `occurred_at` (the row's own timestamp), which together drive both
/// union ordering for append-only aggregates and last-writer-wins for
/// mutable ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncEvent {
    pub aggregate: String,
    pub aggregate_id: String,
    pub seq: i64,
    /// The change's own timestamp, used as the LWW clock for mutable
    /// aggregates. For a task this is `updated_at`; for a message (which
    /// never merges by LWW) it is informational only.
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at: OffsetDateTime,
    /// The opaque per-aggregate payload (the store's `event_json`); the
    /// applier interprets it by `aggregate`.
    pub payload: serde_json::Value,
}

impl SyncEvent {
    /// True for aggregates that merge by union (append-only); their
    /// inbound apply is an idempotent insert, never an LWW comparison.
    pub fn is_append_only(&self) -> bool {
        self.aggregate == "message"
    }
}
