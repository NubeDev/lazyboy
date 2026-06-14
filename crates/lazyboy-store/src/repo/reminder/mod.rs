//! Reminders: time-anchored prompts in a space's durable memory
//! (SCOPE.md build step 4). `create` schedules one; `list` reads a
//! space's reminders; `set_status` fires or dismisses; `due` is the
//! firing pass's query for pending reminders that have come due.

mod create;
mod due;
mod get;
mod list;
mod set_status;

pub use create::{create, NewReminder};
pub use due::due;
pub use get::get;
pub use list::list;
pub use set_status::set_status;
