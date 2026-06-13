use crate::{Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// Grant a user or group a role in a space. `principal_kind` is `user`
/// or `group`; `principal_id` is the matching identity/group id as text
/// because it spans two aggregate types. The first structure past
/// single-tenancy; modeled, not enforced in MVP under R4
/// (DOCS/WORKFLOWS.md).
pub async fn grant_membership(
    store: &Store,
    space_id: Id<Space>,
    principal_kind: &str,
    principal_id: &str,
    role: &str,
) -> Result<String, StoreError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO space_memberships (id, space_id, principal_kind, principal_id, role) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(space_id.to_string())
    .bind(principal_kind)
    .bind(principal_id)
    .bind(role)
    .execute(store.pool())
    .await?;
    Ok(id)
}
