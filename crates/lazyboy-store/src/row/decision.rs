use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use time::OffsetDateTime;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{Decision, Identity, Message, Space};
use lazyboy_types::Id;

/// A decoded `decisions` row — a durable record of something the space
/// settled, anchored to the message it was decided in.
#[derive(Debug, Clone)]
pub struct DecisionRow {
    pub id: Id<Decision>,
    pub space_id: Id<Space>,
    pub message_id: Option<Id<Message>>,
    pub summary: String,
    pub decided_by_identity_id: Option<Id<Identity>>,
    pub decided_at: OffsetDateTime,
}

impl DecisionRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        let message_id: Option<String> = row.try_get("message_id")?;
        let decided_by: Option<String> = row.try_get("decided_by_identity_id")?;
        Ok(Self {
            id: decode::id(row.try_get("id")?, "decisions.id")?,
            space_id: decode::id(row.try_get("space_id")?, "decisions.space_id")?,
            message_id: message_id
                .map(|v| decode::id(&v, "decisions.message_id"))
                .transpose()?,
            summary: row.try_get("summary")?,
            decided_by_identity_id: decided_by
                .map(|v| decode::id(&v, "decisions.decided_by_identity_id"))
                .transpose()?,
            decided_at: decode::ts(row.try_get("decided_at")?, "decisions.decided_at")?,
        })
    }
}
