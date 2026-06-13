use sqlx::Row;
use uuid::Uuid;

use crate::{Store, StoreError};
use lazyboy_types::domain::Identity;
use lazyboy_types::Id;

/// The first identity of a given `kind` (e.g. `agent`, `human`) in the
/// node. The server resolves the principals that author runs and
/// decisions this way instead of carrying the CLI's config sidecar:
/// SQLite stays the source of truth (SCOPE.md R1). Returns `None` if the
/// node has not been bootstrapped with that principal yet.
pub async fn find_by_kind(store: &Store, kind: &str) -> Result<Option<Id<Identity>>, StoreError> {
    let row = sqlx::query("SELECT id FROM identities WHERE kind = ? ORDER BY id LIMIT 1")
        .bind(kind)
        .fetch_optional(store.pool())
        .await?;
    match row {
        Some(row) => {
            let raw: String = row.try_get("id")?;
            let id = Uuid::parse_str(&raw)
                .map(Id::from_uuid)
                .map_err(|e| StoreError::Decode {
                    column: "identities.id",
                    detail: e.to_string(),
                })?;
            Ok(Some(id))
        }
        None => Ok(None),
    }
}
