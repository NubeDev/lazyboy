use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::CalendarEventDto;

/// Optional `?from=&to=` RFC3339 bounds on event start time.
#[derive(Deserialize)]
pub struct CalendarQuery {
    from: Option<String>,
    to: Option<String>,
}

/// `GET /spaces/:id/calendar` -> `CalendarEvent[]`, optionally windowed
/// by `?from`/`?to`. A malformed bound is a 400.
pub async fn list_calendar(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
    Query(query): Query<CalendarQuery>,
) -> Result<Json<Vec<CalendarEventDto>>, ApiError> {
    let parse = |label: &str, v: Option<String>| -> Result<Option<OffsetDateTime>, ApiError> {
        v.map(|s| OffsetDateTime::parse(&s, &Rfc3339))
            .transpose()
            .map_err(|e| ApiError::BadRequest(format!("{label} not rfc3339: {e}")))
    };
    let window = repo::calendar::Window {
        from: parse("from", query.from)?,
        to: parse("to", query.to)?,
    };
    let rows = repo::calendar::list(state.store(), space_id, window).await?;
    Ok(Json(rows.into_iter().map(CalendarEventDto::from).collect()))
}
