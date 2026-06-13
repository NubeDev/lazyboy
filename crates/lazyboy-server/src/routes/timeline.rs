use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::MessageDto;

/// `GET /spaces/:id/timeline` -> `Message[]` (RpcClient.timeline).
pub async fn timeline(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Result<Json<Vec<MessageDto>>, ApiError> {
    let rows = repo::message::list(state.store(), space_id).await?;
    Ok(Json(rows.into_iter().map(MessageDto::from).collect()))
}
