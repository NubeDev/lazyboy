use sqlx::Acquire;
use sqlx::Row;

use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::OutboxEvent;
use lazyboy_types::Id;

/// A state change to append to the replication outbox. `aggregate`
/// names the table/stream (`message`, `task`, `approval`, ...) and
/// `aggregate_id` the row within it; `event_json` is the serialized
/// change a peer applies on the far side.
pub struct NewOutboxEvent<'a> {
    pub aggregate: &'a str,
    pub aggregate_id: &'a str,
    pub event_json: &'a str,
}

/// Append one event, allocating the next per-aggregate `seq`.
///
/// The seq is `max(seq)+1` scoped to the aggregate, computed and
/// inserted inside one transaction. The read and the insert must be
/// atomic: two concurrent appends that both read the same max would
/// otherwise pick the same seq, and the `UNIQUE(aggregate, seq)` index
/// (0002_domain.sql) would reject the second with a constraint error
/// rather than serializing them. Holding a transaction across the
/// SELECT...INSERT serializes them so each gets a distinct, gapless
/// seq. Seq is per-aggregate, not global: a peer reconstructs each
/// aggregate's order independently, and a union merge of append-only
/// aggregates (messages) needs no global clock.
pub async fn append(store: &Store, ev: NewOutboxEvent<'_>) -> Result<Id<OutboxEvent>, StoreError> {
    let id = Id::<OutboxEvent>::new();
    let mut conn = store.pool().acquire().await?;
    let mut tx = conn.begin().await?;

    let next_seq: i64 = sqlx::query(
        "SELECT COALESCE(MAX(seq), 0) + 1 AS next FROM outbox_events WHERE aggregate = ?",
    )
    .bind(ev.aggregate)
    .fetch_one(&mut *tx)
    .await?
    .try_get("next")?;

    sqlx::query(
        "INSERT INTO outbox_events \
         (id, aggregate, aggregate_id, event_json, seq, created_at, synced_at) \
         VALUES (?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(id.to_string())
    .bind(ev.aggregate)
    .bind(ev.aggregate_id)
    .bind(ev.event_json)
    .bind(next_seq)
    .bind(clock::fmt(clock::now()))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(id)
}
