use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use time::OffsetDateTime;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::OutboxEvent;
use lazyboy_types::Id;

/// A decoded `outbox_events` row: one appended state change awaiting
/// (or having completed) replication over the Zenoh fabric. The pair
/// `aggregate` and `aggregate_id` name what changed; `seq` is the
/// per-aggregate monotonic order; `synced_at` is `None` until shipped.
#[derive(Debug, Clone)]
pub struct OutboxEventRow {
    pub id: Id<OutboxEvent>,
    pub aggregate: String,
    pub aggregate_id: String,
    pub event_json: String,
    pub seq: i64,
    pub created_at: OffsetDateTime,
    pub synced_at: Option<OffsetDateTime>,
}

impl OutboxEventRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        let synced_at = row
            .try_get::<Option<String>, _>("synced_at")?
            .map(|v| decode::ts(&v, "outbox_events.synced_at"))
            .transpose()?;
        Ok(Self {
            id: decode::id(row.try_get("id")?, "outbox_events.id")?,
            aggregate: row.try_get("aggregate")?,
            aggregate_id: row.try_get("aggregate_id")?,
            event_json: row.try_get("event_json")?,
            seq: row.try_get("seq")?,
            created_at: decode::ts(row.try_get("created_at")?, "outbox_events.created_at")?,
            synced_at,
        })
    }
}
