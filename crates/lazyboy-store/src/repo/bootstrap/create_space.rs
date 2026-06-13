use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{Space, Workspace};
use lazyboy_types::Id;

/// Insert a space (one idea/initiative) under a workspace, starting
/// `active`. The (workspace_id, slug) uniqueness is enforced in SQL.
pub async fn create_space(
    store: &Store,
    workspace_id: Id<Workspace>,
    slug: &str,
    title: &str,
) -> Result<Id<Space>, StoreError> {
    let id = Id::<Space>::new();
    sqlx::query(
        "INSERT INTO spaces (id, workspace_id, slug, title, status, created_at) \
         VALUES (?, ?, ?, ?, 'active', ?)",
    )
    .bind(id.to_string())
    .bind(workspace_id.to_string())
    .bind(slug)
    .bind(title)
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(id)
}
