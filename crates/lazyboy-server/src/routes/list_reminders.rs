use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::ReminderDto;

/// `GET /spaces/:id/reminders` -> `Reminder[]`.
pub async fn list_reminders(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Result<Json<Vec<ReminderDto>>, ApiError> {
    let rows = repo::reminder::list(state.store(), space_id).await?;
    Ok(Json(rows.into_iter().map(ReminderDto::from).collect()))
}
