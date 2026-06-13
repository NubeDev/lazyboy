use crate::{ReminderRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// A space's reminders ordered by when they fire, soonest first.
pub async fn list(store: &Store, space_id: Id<Space>) -> Result<Vec<ReminderRow>, StoreError> {
    let rows = sqlx::query("SELECT * FROM reminders WHERE space_id = ? ORDER BY due_at, id")
        .bind(space_id.to_string())
        .fetch_all(store.pool())
        .await?;
    rows.iter().map(ReminderRow::from_row).collect()
}
