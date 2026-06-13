use axum::extract::{Path, State};
use axum::Json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CalendarEventDto, UpsertCalendarBody};

/// `POST /spaces/:id/calendar` body `{source, title, starts_at,
/// external_ref?, ends_at?, meta_json?}` -> the upserted event. A synced
/// event re-POSTed with the same `(source, external_ref)` refreshes the
/// existing row rather than doubling it. Timestamps are RFC3339; a
/// malformed one is a 400.
pub async fn upsert_calendar(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
    Json(body): Json<UpsertCalendarBody>,
) -> Result<Json<CalendarEventDto>, ApiError> {
    let starts_at = OffsetDateTime::parse(&body.starts_at, &Rfc3339)
        .map_err(|e| ApiError::BadRequest(format!("starts_at not rfc3339: {e}")))?;
    let ends_at = body
        .ends_at
        .as_deref()
        .map(|s| OffsetDateTime::parse(s, &Rfc3339))
        .transpose()
        .map_err(|e| ApiError::BadRequest(format!("ends_at not rfc3339: {e}")))?;
    let id = repo::calendar::upsert(
        state.store(),
        repo::calendar::NewCalendarEvent {
            space_id,
            source: &body.source,
            external_ref: body.external_ref.as_deref(),
            title: &body.title,
            starts_at,
            ends_at,
            meta_json: body.meta_json.as_deref(),
        },
    )
    .await?;
    let rows =
        repo::calendar::list(state.store(), space_id, repo::calendar::Window::default()).await?;
    let row = rows
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| ApiError::NotFound("calendar event vanished after upsert".to_owned()))?;
    Ok(Json(row.into()))
}
