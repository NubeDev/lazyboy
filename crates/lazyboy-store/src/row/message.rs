use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use time::OffsetDateTime;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{Message, MessageKind, Space};
use lazyboy_types::Id;

/// A decoded `messages` row.
#[derive(Debug, Clone)]
pub struct MessageRow {
    pub id: Id<Message>,
    pub space_id: Id<Space>,
    pub kind: MessageKind,
    pub body: String,
    pub ts: OffsetDateTime,
    pub ref_id: Option<String>,
}

impl MessageRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "messages.id")?,
            space_id: decode::id(row.try_get("space_id")?, "messages.space_id")?,
            kind: decode::parse(row.try_get("kind")?, "messages.kind")?,
            body: row.try_get("body")?,
            ts: decode::ts(row.try_get("ts")?, "messages.ts")?,
            ref_id: row.try_get("ref_id")?,
        })
    }
}
