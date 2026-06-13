use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{Space, Workspace};
use lazyboy_types::Id;

/// A decoded `spaces` row — one idea/initiative under a workspace.
#[derive(Debug, Clone)]
pub struct SpaceRow {
    pub id: Id<Space>,
    pub workspace_id: Id<Workspace>,
    pub slug: String,
    pub title: String,
    pub status: String,
}

impl SpaceRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "spaces.id")?,
            workspace_id: decode::id(row.try_get("workspace_id")?, "spaces.workspace_id")?,
            slug: row.try_get("slug")?,
            title: row.try_get("title")?,
            status: row.try_get("status")?,
        })
    }
}
