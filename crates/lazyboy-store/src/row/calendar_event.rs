use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use time::OffsetDateTime;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{CalendarEvent, Space};
use lazyboy_types::Id;

/// A decoded `calendar_events` row. `source`/`external_ref` carry the
/// provenance of a synced event (gcal and re-syncs dedup on them); a
/// `local` event has no `external_ref`.
#[derive(Debug, Clone)]
pub struct CalendarEventRow {
    pub id: Id<CalendarEvent>,
    pub space_id: Id<Space>,
    pub source: String,
    pub external_ref: Option<String>,
    pub title: String,
    pub starts_at: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub meta_json: Option<String>,
}

impl CalendarEventRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        let ends_at: Option<String> = row.try_get("ends_at")?;
        Ok(Self {
            id: decode::id(row.try_get("id")?, "calendar_events.id")?,
            space_id: decode::id(row.try_get("space_id")?, "calendar_events.space_id")?,
            source: row.try_get("source")?,
            external_ref: row.try_get("external_ref")?,
            title: row.try_get("title")?,
            starts_at: decode::ts(row.try_get("starts_at")?, "calendar_events.starts_at")?,
            ends_at: ends_at
                .map(|v| decode::ts(&v, "calendar_events.ends_at"))
                .transpose()?,
            meta_json: row.try_get("meta_json")?,
        })
    }
}
