use sqlx::Row;

use crate::{Store, StoreError};
use lazyboy_types::domain::{Group, Identity};
use lazyboy_types::Id;

/// The identities in a group, ordered for a stable listing. Used by
/// tests and the modeled (not enforced) membership surface.
pub async fn list_members(
    store: &Store,
    group_id: Id<Group>,
) -> Result<Vec<Id<Identity>>, StoreError> {
    let rows = sqlx::query(
        "SELECT identity_id FROM group_members WHERE group_id = ? ORDER BY identity_id",
    )
    .bind(group_id.to_string())
    .fetch_all(store.pool())
    .await?;
    rows.iter()
        .map(|row| {
            let raw: String = row.try_get("identity_id")?;
            uuid::Uuid::parse_str(&raw)
                .map(Id::from_uuid)
                .map_err(|e| StoreError::Decode {
                    column: "group_members.identity_id",
                    detail: e.to_string(),
                })
        })
        .collect()
}
