use crate::{MessageRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// Read a space's timeline in append order. This is what the UI
/// subscribes to render the thread; it holds no authoritative state
/// of its own (SCOPE.md R1).
pub async fn list(store: &Store, space_id: Id<Space>) -> Result<Vec<MessageRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM messages WHERE space_id = ? ORDER BY ts, id")
        .bind(space_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(MessageRow::from_row).collect()
}
