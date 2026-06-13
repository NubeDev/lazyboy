use crate::{Store, StoreError};
use lazyboy_types::domain::{Group, Identity};
use lazyboy_types::Id;

/// Add an identity to a group. The composite PK makes a repeated add a
/// no-op rather than a duplicate. Modeled, not enforced in MVP (R4,
/// DOCS/WORKFLOWS.md).
pub async fn add_member(
    store: &Store,
    group_id: Id<Group>,
    identity_id: Id<Identity>,
) -> Result<(), StoreError> {
    sqlx::query("INSERT OR IGNORE INTO group_members (group_id, identity_id) VALUES (?, ?)")
        .bind(group_id.to_string())
        .bind(identity_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
