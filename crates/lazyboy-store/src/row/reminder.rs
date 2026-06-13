use sqlx::sqlite::SqliteRow;
use sqlx::Row;
use time::OffsetDateTime;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{Reminder, ReminderStatus, Space, Task};
use lazyboy_types::Id;

/// A decoded `reminders` row. A reminder may stand alone or hang off a
/// task; the firing pass reads `due_at` and `status` to decide what to
/// surface.
#[derive(Debug, Clone)]
pub struct ReminderRow {
    pub id: Id<Reminder>,
    pub space_id: Id<Space>,
    pub task_id: Option<Id<Task>>,
    pub due_at: OffsetDateTime,
    pub body: String,
    pub status: ReminderStatus,
}

impl ReminderRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        let task_id: Option<String> = row.try_get("task_id")?;
        Ok(Self {
            id: decode::id(row.try_get("id")?, "reminders.id")?,
            space_id: decode::id(row.try_get("space_id")?, "reminders.space_id")?,
            task_id: task_id
                .map(|v| decode::id(&v, "reminders.task_id"))
                .transpose()?,
            due_at: decode::ts(row.try_get("due_at")?, "reminders.due_at")?,
            body: row.try_get("body")?,
            status: decode::parse(row.try_get("status")?, "reminders.status")?,
        })
    }
}
