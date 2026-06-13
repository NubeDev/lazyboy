use crate::{Store, StoreError};
use lazyboy_types::domain::{Group, Workspace};
use lazyboy_types::Id;

/// Create a named group of identities inside the workspace. Part of the
/// membership model; modeled, not enforced in MVP under R4
/// (DOCS/WORKFLOWS.md).
pub async fn create_group(
    store: &Store,
    workspace_id: Id<Workspace>,
    name: &str,
) -> Result<Id<Group>, StoreError> {
    let id = Id::<Group>::new();
    sqlx::query("INSERT INTO groups (id, workspace_id, name) VALUES (?, ?, ?)")
        .bind(id.to_string())
        .bind(workspace_id.to_string())
        .bind(name)
        .execute(store.pool())
        .await?;
    Ok(id)
}
