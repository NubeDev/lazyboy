use time::OffsetDateTime;

use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{CalendarEvent, Space};
use lazyboy_types::Id;

/// A calendar event arriving into a space. A synced source (gcal) can
/// re-deliver the same event, so it carries an `external_ref` the upsert
/// dedups on; a `local` event with no ref is always a fresh insert.
pub struct NewCalendarEvent<'a> {
    pub space_id: Id<Space>,
    pub source: &'a str,
    pub external_ref: Option<&'a str>,
    pub title: &'a str,
    pub starts_at: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub meta_json: Option<&'a str>,
}

/// Insert the event, or update the existing one identified by
/// `(space_id, source, external_ref)` when the source re-syncs it — the
/// same idempotency boundary ingress uses, so a redelivery refreshes the
/// row instead of doubling it. Returns the row's id either way.
pub async fn upsert(
    store: &Store,
    new: NewCalendarEvent<'_>,
) -> Result<Id<CalendarEvent>, StoreError> {
    if let Some(external_ref) = new.external_ref {
        if let Some(existing) = find(store, new.space_id, new.source, external_ref).await? {
            sqlx::query(
                "UPDATE calendar_events SET title = ?, starts_at = ?, ends_at = ?, meta_json = ? \
                 WHERE id = ?",
            )
            .bind(new.title)
            .bind(clock::fmt(new.starts_at))
            .bind(new.ends_at.map(clock::fmt))
            .bind(new.meta_json)
            .bind(existing.to_string())
            .execute(store.pool())
            .await?;
            return Ok(existing);
        }
    }
    let id = Id::<CalendarEvent>::new();
    sqlx::query(
        "INSERT INTO calendar_events (id, space_id, source, external_ref, title, starts_at, \
         ends_at, meta_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(new.space_id.to_string())
    .bind(new.source)
    .bind(new.external_ref)
    .bind(new.title)
    .bind(clock::fmt(new.starts_at))
    .bind(new.ends_at.map(clock::fmt))
    .bind(new.meta_json)
    .execute(store.pool())
    .await?;
    Ok(id)
}

async fn find(
    store: &Store,
    space_id: Id<Space>,
    source: &str,
    external_ref: &str,
) -> Result<Option<Id<CalendarEvent>>, StoreError> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id FROM calendar_events WHERE space_id = ? AND source = ? AND external_ref = ? \
         LIMIT 1",
    )
    .bind(space_id.to_string())
    .bind(source)
    .bind(external_ref)
    .fetch_optional(store.pool())
    .await?;
    match row {
        None => Ok(None),
        Some(row) => {
            let id: String = row.try_get("id")?;
            uuid::Uuid::parse_str(&id)
                .map(|u| Some(Id::from_uuid(u)))
                .map_err(|e| StoreError::Decode {
                    column: "calendar_events.id",
                    detail: e.to_string(),
                })
        }
    }
}
