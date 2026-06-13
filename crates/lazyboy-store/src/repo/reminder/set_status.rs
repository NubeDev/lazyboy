use crate::{Store, StoreError};
use lazyboy_types::domain::{Reminder, ReminderStatus};
use lazyboy_types::Id;

/// Move a reminder to a new status: the firing pass marks it `fired`, a
/// human `dismissed`. Returns whether a row was changed, so a dismiss of
/// an already-settled reminder is a reportable no-op rather than silent.
pub async fn set_status(
    store: &Store,
    reminder_id: Id<Reminder>,
    status: ReminderStatus,
) -> Result<bool, StoreError> {
    let result = sqlx::query("UPDATE reminders SET status = ? WHERE id = ?")
        .bind(status.as_str())
        .bind(reminder_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(result.rows_affected() > 0)
}
