use crate::{RunRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// Every agent run in a space, oldest first. Feeds the right-panel run
/// list of the cowork UI.
pub async fn list(store: &Store, space_id: Id<Space>) -> Result<Vec<RunRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM agent_runs WHERE space_id = ? ORDER BY started_at, id")
        .bind(space_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(RunRow::from_row).collect()
}
