use crate::{Store, StoreError};
use lazyboy_types::domain::{Identity, Workspace};
use lazyboy_types::Id;

/// A timeline author. `kind` is free text for MVP (`human`, `agent`,
/// or an integration principal); attribution matters even single-tenant
/// because P2P timelines carry authorship (SCOPE.md).
pub struct NewIdentity<'a> {
    pub kind: &'a str,
    pub display_name: &'a str,
    pub external_ref: Option<&'a str>,
}

pub async fn create_identity(
    store: &Store,
    workspace_id: Id<Workspace>,
    new: NewIdentity<'_>,
) -> Result<Id<Identity>, StoreError> {
    let id = Id::<Identity>::new();
    sqlx::query(
        "INSERT INTO identities (id, workspace_id, kind, display_name, external_ref) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(workspace_id.to_string())
    .bind(new.kind)
    .bind(new.display_name)
    .bind(new.external_ref)
    .execute(store.pool())
    .await?;
    Ok(id)
}
