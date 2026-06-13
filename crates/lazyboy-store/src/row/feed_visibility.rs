use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{Integration, Space};
use lazyboy_types::Id;

/// A decoded `feed_visibility` row — per (feed, space, principal) access
/// to a feed inside a space (SCOPE.md "Feed visibility"). `mode` is
/// `visible` or `hidden`. The most significant departure from "everyone
/// sees everything," modeled but kept out of the MVP trust gate (R4,
/// DOCS/WORKFLOWS.md).
#[derive(Debug, Clone)]
pub struct FeedVisibilityRow {
    pub id: String,
    pub feed_integration_id: Id<Integration>,
    pub space_id: Id<Space>,
    pub principal_kind: String,
    pub principal_id: String,
    pub mode: String,
}

impl FeedVisibilityRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: row.try_get("id")?,
            feed_integration_id: decode::id(
                row.try_get("feed_integration_id")?,
                "feed_visibility.feed_integration_id",
            )?,
            space_id: decode::id(row.try_get("space_id")?, "feed_visibility.space_id")?,
            principal_kind: row.try_get("principal_kind")?,
            principal_id: row.try_get("principal_id")?,
            mode: row.try_get("mode")?,
        })
    }
}
