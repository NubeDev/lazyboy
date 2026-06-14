use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::{Reminder, ReminderStatus};
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::ReminderDto;

/// `POST /reminders/:id/dismiss` -> the dismissed `Reminder`. A dismiss
/// of an unknown or already-settled reminder is a 404, so a racing
/// second click is reported rather than silently succeeding. The settled
/// row is returned so the client updates in place without a re-list.
pub async fn dismiss_reminder(
    State(state): State<AppState>,
    Path(reminder_id): Path<Id<Reminder>>,
) -> Result<Json<ReminderDto>, ApiError> {
    let changed =
        repo::reminder::set_status(state.store(), reminder_id, ReminderStatus::Dismissed).await?;
    if !changed {
        return Err(ApiError::NotFound("reminder not found".to_owned()));
    }
    let row = repo::reminder::get(state.store(), reminder_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("reminder vanished after dismiss".to_owned()))?;
    Ok(Json(row.into()))
}
