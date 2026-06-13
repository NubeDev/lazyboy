use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// A decoded `space_memberships` row — a user or group granted a role in
/// a space. `principal_kind` is `user` or `group`; `principal_id` is the
/// matching identity/group id, kept as text because it crosses two
/// aggregate types. Modeled, not enforced in MVP (R4, DOCS/WORKFLOWS.md).
#[derive(Debug, Clone)]
pub struct SpaceMembershipRow {
    pub id: String,
    pub space_id: Id<Space>,
    pub principal_kind: String,
    pub principal_id: String,
    pub role: String,
}

impl SpaceMembershipRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: row.try_get("id")?,
            space_id: decode::id(row.try_get("space_id")?, "space_memberships.space_id")?,
            principal_kind: row.try_get("principal_kind")?,
            principal_id: row.try_get("principal_id")?,
            role: row.try_get("role")?,
        })
    }
}
