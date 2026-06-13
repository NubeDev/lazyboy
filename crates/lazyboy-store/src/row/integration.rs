use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{Integration, Provider, Workspace};
use lazyboy_types::Id;

/// A decoded `integrations` row — one external feed bound to the
/// workspace. `secret_ref` names a host secrets-store entry, never raw
/// creds (SCOPE.md R5); `config_json` carries the explicit ingress
/// bindings (which space a repo/label/thread/channel routes into).
#[derive(Debug, Clone)]
pub struct IntegrationRow {
    pub id: Id<Integration>,
    pub workspace_id: Id<Workspace>,
    pub provider: Provider,
    pub account_ref: Option<String>,
    pub secret_ref: Option<String>,
    pub status: String,
    pub config_json: Option<String>,
}

impl IntegrationRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "integrations.id")?,
            workspace_id: decode::id(row.try_get("workspace_id")?, "integrations.workspace_id")?,
            provider: decode::parse(row.try_get("provider")?, "integrations.provider")?,
            account_ref: row.try_get("account_ref")?,
            secret_ref: row.try_get("secret_ref")?,
            status: row.try_get("status")?,
            config_json: row.try_get("config_json")?,
        })
    }
}
