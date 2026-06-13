use time::OffsetDateTime;

use crate::repo::clock;
use crate::{ReminderRow, Store, StoreError};

/// Pending reminders whose `due_at` has arrived, soonest first — the
/// query the firing pass runs to decide what to surface. `as_of` is
/// passed rather than read from the clock so firing is testable.
pub async fn due(store: &Store, as_of: OffsetDateTime) -> Result<Vec<ReminderRow>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM reminders WHERE status = 'pending' AND due_at <= ? ORDER BY due_at, id",
    )
    .bind(clock::fmt(as_of))
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(ReminderRow::from_row).collect()
}
