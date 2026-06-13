use sqlx::Row;

use crate::repo::clock;
use crate::repo::message::{self, NewMessage};
use crate::{Store, StoreError};
use lazyboy_types::domain::{Identity, IngressEvent, Integration, Message, MessageKind, Space};
use lazyboy_types::Id;

/// A normalized external event ready to land in a space. `external_id`
/// is the provider's stable id for the event; it is the dedup key the
/// `UNIQUE(integration_id, external_id)` constraint enforces. `body` is
/// the human-readable text the timeline message carries.
pub struct NewIngress<'a> {
    pub integration_id: Id<Integration>,
    pub space_id: Id<Space>,
    pub author: Id<Identity>,
    pub external_id: &'a str,
    pub kind: &'a str,
    pub payload_json: &'a str,
    pub body: &'a str,
}

/// The result of an ingest: the timeline message the event maps to, and
/// whether this call was a redelivery of an `external_id` already seen.
pub struct IngestOutcome {
    pub message_id: Id<Message>,
    pub deduped: bool,
}

/// Land an external event as an `ingress` message in its bound space,
/// idempotently.
///
/// The dedup invariant: an `(integration_id, external_id)` pair maps to
/// at most one timeline message, forever. A provider can redeliver the
/// same webhook or a poll can re-observe the same item; without this the
/// timeline would double the message. We check `ingress_events` first
/// and, on a hit, return the existing `message_id` without appending —
/// so redelivery is a no-op observable only as `deduped: true`. The
/// `UNIQUE(integration_id, external_id)` constraint is the backstop: it
/// turns a lost race (two concurrent deliveries past the check) into an
/// insert error rather than a second message.
///
/// On a miss this is a two-step sequential write (append message, then
/// insert the ingress row referencing it). The crate does not use
/// transactions elsewhere, so this matches existing style; the ingress
/// row is the audit record, and a crash between the two steps leaves an
/// orphan message rather than a lost event, which is the safe direction.
pub async fn ingest(store: &Store, new: NewIngress<'_>) -> Result<IngestOutcome, StoreError> {
    if let Some(existing) = existing_message(store, new.integration_id, new.external_id).await? {
        return Ok(IngestOutcome {
            message_id: existing,
            deduped: true,
        });
    }

    let message_id = message::append(
        store,
        NewMessage {
            space_id: new.space_id,
            author: new.author,
            kind: MessageKind::Ingress,
            body: new.body,
            ref_id: None,
        },
    )
    .await?;

    let id = Id::<IngressEvent>::new();
    sqlx::query(
        "INSERT INTO ingress_events (id, integration_id, space_id, external_id, kind, \
         payload_json, message_id, received_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(new.integration_id.to_string())
    .bind(new.space_id.to_string())
    .bind(new.external_id)
    .bind(new.kind)
    .bind(new.payload_json)
    .bind(message_id.to_string())
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;

    Ok(IngestOutcome {
        message_id,
        deduped: false,
    })
}

async fn existing_message(
    store: &Store,
    integration_id: Id<Integration>,
    external_id: &str,
) -> Result<Option<Id<Message>>, StoreError> {
    let row = sqlx::query(
        "SELECT message_id FROM ingress_events WHERE integration_id = ? AND external_id = ? \
         LIMIT 1",
    )
    .bind(integration_id.to_string())
    .bind(external_id)
    .fetch_optional(store.pool())
    .await?;
    match row {
        None => Ok(None),
        Some(row) => {
            let raw: Option<String> = row.try_get("message_id")?;
            raw.map(|v| {
                uuid::Uuid::parse_str(&v)
                    .map(Id::from_uuid)
                    .map_err(|e| StoreError::Decode {
                        column: "ingress_events.message_id",
                        detail: e.to_string(),
                    })
            })
            .transpose()
        }
    }
}
