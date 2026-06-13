use time::OffsetDateTime;

use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{Reminder, ReminderStatus, Space, Task};
use lazyboy_types::Id;

/// A reminder to schedule. `due_at` is caller-supplied (the moment to
/// fire), unlike the timeline's append timestamps; a reminder starts
/// `pending` and may optionally hang off a task.
pub struct NewReminder<'a> {
    pub space_id: Id<Space>,
    pub task_id: Option<Id<Task>>,
    pub due_at: OffsetDateTime,
    pub body: &'a str,
}

pub async fn create(store: &Store, new: NewReminder<'_>) -> Result<Id<Reminder>, StoreError> {
    let id = Id::<Reminder>::new();
    sqlx::query(
        "INSERT INTO reminders (id, space_id, task_id, due_at, body, status) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(new.space_id.to_string())
    .bind(new.task_id.map(|t| t.to_string()))
    .bind(clock::fmt(new.due_at))
    .bind(new.body)
    .bind(ReminderStatus::Pending.as_str())
    .execute(store.pool())
    .await?;
    Ok(id)
}
