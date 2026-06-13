use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

/// Insert the node's workspace row and return its id.
pub async fn create_workspace(store: &Store, name: &str) -> Result<Id<Workspace>, StoreError> {
    let id = Id::<Workspace>::new();
    sqlx::query("INSERT INTO workspaces (id, name, created_at) VALUES (?, ?, ?)")
        .bind(id.to_string())
        .bind(name)
        .bind(clock::fmt(clock::now()))
        .execute(store.pool())
        .await?;
    Ok(id)
}
