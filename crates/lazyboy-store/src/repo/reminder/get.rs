use crate::{ReminderRow, Store, StoreError};
use lazyboy_types::domain::Reminder;
use lazyboy_types::Id;

/// A single reminder by id, or `None` if it does not exist. Used by the
/// shells to project the row a dismiss just settled back to the UI.
pub async fn get(
    store: &Store,
    reminder_id: Id<Reminder>,
) -> Result<Option<ReminderRow>, StoreError> {
    let row = sqlx::query("SELECT * FROM reminders WHERE id = ?")
        .bind(reminder_id.to_string())
        .fetch_optional(store.pool())
        .await?;
    row.as_ref().map(ReminderRow::from_row).transpose()
}
