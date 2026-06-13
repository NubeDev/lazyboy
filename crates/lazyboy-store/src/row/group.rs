use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{Group, Workspace};
use lazyboy_types::Id;

/// A decoded `groups` row — a named set of identities inside the
/// workspace trust boundary. Part of the membership model that is
/// modeled but not enforced in MVP code under R4 (DOCS/WORKFLOWS.md).
#[derive(Debug, Clone)]
pub struct GroupRow {
    pub id: Id<Group>,
    pub workspace_id: Id<Workspace>,
    pub name: String,
}

impl GroupRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "groups.id")?,
            workspace_id: decode::id(row.try_get("workspace_id")?, "groups.workspace_id")?,
            name: row.try_get("name")?,
        })
    }
}
