use uuid::Uuid;

use crate::{Store, StoreError};
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

/// The node's sole workspace id. A node bootstraps exactly one workspace
/// (SCOPE R5), so the oldest row is that workspace; an empty table means
/// bootstrap never ran.
pub async fn current(store: &Store) -> Result<Id<Workspace>, StoreError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT id FROM workspaces ORDER BY created_at, id LIMIT 1")
            .fetch_optional(store.pool())
            .await?;
    let (id,) =
        row.ok_or_else(|| StoreError::NotFound("no workspace; node not bootstrapped".to_owned()))?;
    Uuid::parse_str(&id).map(Id::from_uuid).map_err(|e| StoreError::Decode {
        column: "workspaces.id",
        detail: e.to_string(),
    })
}
