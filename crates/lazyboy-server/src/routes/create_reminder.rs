use axum::extract::{Path, State};
use axum::Json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreateReminderBody, ReminderDto};

/// `POST /spaces/:id/reminders` body `{body, due_at, task_id?}` -> the
/// created `Reminder`. `due_at` is RFC3339; a malformed instant is a 400.
pub async fn create_reminder(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
    Json(body): Json<CreateReminderBody>,
) -> Result<Json<ReminderDto>, ApiError> {
    let due_at = OffsetDateTime::parse(&body.due_at, &Rfc3339)
        .map_err(|e| ApiError::BadRequest(format!("due_at not rfc3339: {e}")))?;
    let id = repo::reminder::create(
        state.store(),
        repo::reminder::NewReminder {
            space_id,
            task_id: body.task_id,
            due_at,
            body: &body.body,
        },
    )
    .await?;
    let rows = repo::reminder::list(state.store(), space_id).await?;
    let row = rows
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| ApiError::NotFound("reminder vanished after create".to_owned()))?;
    Ok(Json(row.into()))
}
