use crate::{IntegrationRow, Store, StoreError};
use lazyboy_types::domain::Integration;
use lazyboy_types::Id;

/// One integration by id. The ingress sink reads it to resolve the
/// provider and the explicit space bindings in `config_json`.
pub async fn get(store: &Store, id: Id<Integration>) -> Result<Option<IntegrationRow>, StoreError> {
    let row = sqlx::query("SELECT * FROM integrations WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(store.pool())
        .await?;
    row.as_ref().map(IntegrationRow::from_row).transpose()
}
