use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use time::OffsetDateTime;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{IngressEvent, Integration, Message, Space};
use lazyboy_types::Id;

/// A decoded `ingress_events` row — one raw external event, deduped by
/// `(integration_id, external_id)` and mapped to the timeline message it
/// became. This is the idempotency + audit boundary for ingress
/// (SCOPE.md): the row records that an `external_id` was already seen so
/// a redelivery resolves to the existing `message_id` instead of
/// appending a second message.
#[derive(Debug, Clone)]
pub struct IngressEventRow {
    pub id: Id<IngressEvent>,
    pub integration_id: Id<Integration>,
    pub space_id: Id<Space>,
    pub external_id: String,
    pub kind: String,
    pub payload_json: String,
    pub message_id: Option<Id<Message>>,
    pub received_at: OffsetDateTime,
}

impl IngressEventRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        let message_id: Option<String> = row.try_get("message_id")?;
        Ok(Self {
            id: decode::id(row.try_get("id")?, "ingress_events.id")?,
            integration_id: decode::id(
                row.try_get("integration_id")?,
                "ingress_events.integration_id",
            )?,
            space_id: decode::id(row.try_get("space_id")?, "ingress_events.space_id")?,
            external_id: row.try_get("external_id")?,
            kind: row.try_get("kind")?,
            payload_json: row.try_get("payload_json")?,
            message_id: message_id
                .map(|v| decode::id(&v, "ingress_events.message_id"))
                .transpose()?,
            received_at: decode::ts(row.try_get("received_at")?, "ingress_events.received_at")?,
        })
    }
}
