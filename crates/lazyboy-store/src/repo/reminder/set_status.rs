use crate::{Store, StoreError};
use lazyboy_types::domain::{Reminder, ReminderStatus};
use lazyboy_types::Id;

/// Move a reminder to a new status: the firing pass marks it `fired`, a
/// human `dismissed`. The `status != ?` guard means SQLite's
/// `rows_affected` counts an actual transition, not a row merely matched:
/// re-setting a reminder to the status it already holds reports `false`,
/// so a racing second dismiss is a reportable no-op rather than silently
/// succeeding (SQLite counts matched, not changed, rows otherwise).
pub async fn set_status(
    store: &Store,
    reminder_id: Id<Reminder>,
    status: ReminderStatus,
) -> Result<bool, StoreError> {
    let result = sqlx::query("UPDATE reminders SET status = ? WHERE id = ? AND status != ?")
        .bind(status.as_str())
        .bind(reminder_id.to_string())
        .bind(status.as_str())
        .execute(store.pool())
        .await?;
    Ok(result.rows_affected() > 0)
}
