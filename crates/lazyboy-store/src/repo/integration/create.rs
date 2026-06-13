use crate::{Store, StoreError};
use lazyboy_types::domain::{Integration, Provider, Workspace};
use lazyboy_types::Id;

/// An integration to register. `secret_ref` is the only credential
/// surface: it names an entry in the host secrets store, never the raw
/// token (SCOPE.md R5). `config_json` carries the explicit ingress
/// bindings (e.g. `{"bindings":[{"repo":"owner/x","space_id":"..."}]}`);
/// MVP routing is explicit binding, not auto-routing.
pub struct NewIntegration<'a> {
    pub workspace_id: Id<Workspace>,
    pub provider: Provider,
    pub account_ref: Option<&'a str>,
    pub secret_ref: Option<&'a str>,
    pub config_json: Option<&'a str>,
}

pub async fn create(store: &Store, new: NewIntegration<'_>) -> Result<Id<Integration>, StoreError> {
    let id = Id::<Integration>::new();
    sqlx::query(
        "INSERT INTO integrations (id, workspace_id, provider, account_ref, secret_ref, status, \
         config_json) VALUES (?, ?, ?, ?, ?, 'active', ?)",
    )
    .bind(id.to_string())
    .bind(new.workspace_id.to_string())
    .bind(new.provider.as_str())
    .bind(new.account_ref)
    .bind(new.secret_ref)
    .bind(new.config_json)
    .execute(store.pool())
    .await?;
    Ok(id)
}
