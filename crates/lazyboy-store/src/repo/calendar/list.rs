use time::OffsetDateTime;

use crate::repo::clock;
use crate::{CalendarEventRow, Store, StoreError};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

/// An inclusive `[from, to]` bound on event start time. Either end is
/// optional, so callers can ask for "everything", "from now on", or a
/// bounded window without separate verbs.
#[derive(Default)]
pub struct Window {
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
}

/// A space's calendar events, earliest first, optionally filtered to a
/// time window on `starts_at`.
pub async fn list(
    store: &Store,
    space_id: Id<Space>,
    window: Window,
) -> Result<Vec<CalendarEventRow>, StoreError> {
    // Bind the window ends through COALESCE-style guards: a NULL bind
    // makes that side of the range unconstrained, keeping one query for
    // all four open/closed combinations.
    let rows = sqlx::query(
        "SELECT * FROM calendar_events WHERE space_id = ? \
         AND (? IS NULL OR starts_at >= ?) AND (? IS NULL OR starts_at <= ?) \
         ORDER BY starts_at, id",
    )
    .bind(space_id.to_string())
    .bind(window.from.map(clock::fmt))
    .bind(window.from.map(clock::fmt))
    .bind(window.to.map(clock::fmt))
    .bind(window.to.map(clock::fmt))
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(CalendarEventRow::from_row).collect()
}
