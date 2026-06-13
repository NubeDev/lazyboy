use std::cmp::Ordering;

use time::OffsetDateTime;

/// The resolution of SCOPE.md Open Question 1 for MVP: mutable rows
/// (task state, approval status) merge by last-writer-wins. The winner
/// is the event with the greater `occurred_at`; a tie (same millisecond
/// across two nodes) breaks deterministically on the higher per-aggregate
/// `seq`, so every node picks the same winner without coordination.
///
/// Append-only aggregates (messages) never reach this function — they
/// union-merge by idempotent insert. See DOCS/ZENOH.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeKey {
    pub occurred_at: OffsetDateTime,
    pub seq: i64,
}

/// Whether `incoming` should overwrite `current` under LWW. A strictly
/// greater key wins; an equal key does not overwrite (idempotent
/// re-delivery of the same event is a no-op).
pub fn incoming_wins(current: MergeKey, incoming: MergeKey) -> bool {
    match incoming.occurred_at.cmp(&current.occurred_at) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => incoming.seq > current.seq,
    }
}
