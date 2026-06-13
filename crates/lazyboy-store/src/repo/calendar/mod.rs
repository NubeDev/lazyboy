//! Calendar events: a space's durable schedule (SCOPE.md build step 4).
//! `upsert` inserts or refreshes a synced event, deduped on
//! `(space, source, external_ref)` like ingress; `list` reads a space's
//! events, optionally bounded to a time window.

mod list;
mod upsert;

pub use list::{list, Window};
pub use upsert::{upsert, NewCalendarEvent};
