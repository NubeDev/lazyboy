use crate::{DecisionRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// A space's decisions, most recent first — the durable record the
/// timeline scrolls back to.
pub async fn list(store: &Store, space_id: Id<Space>) -> Result<Vec<DecisionRow>, StoreError> {
    let rows =
        sqlx::query("SELECT * FROM decisions WHERE space_id = ? ORDER BY decided_at DESC, id")
            .bind(space_id.to_string())
            .fetch_all(store.pool())
            .await?;
    rows.iter().map(DecisionRow::from_row).collect()
}
