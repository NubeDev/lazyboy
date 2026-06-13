use crate::{Store, StoreError};
use lazyboy_types::domain::{Integration, Space};
use lazyboy_types::Id;

/// Set a principal's visibility of a feed inside a space (SCOPE.md
/// "Feed visibility"). `mode` is `visible` or `hidden`; the most
/// significant departure from "everyone sees everything." Modeled, not
/// enforced in the MVP trust gate under R4 (DOCS/WORKFLOWS.md).
pub async fn set_feed_visibility(
    store: &Store,
    feed_integration_id: Id<Integration>,
    space_id: Id<Space>,
    principal_kind: &str,
    principal_id: &str,
    mode: &str,
) -> Result<String, StoreError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO feed_visibility (id, feed_integration_id, space_id, principal_kind, \
         principal_id, mode) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(feed_integration_id.to_string())
    .bind(space_id.to_string())
    .bind(principal_kind)
    .bind(principal_id)
    .bind(mode)
    .execute(store.pool())
    .await?;
    Ok(id)
}
