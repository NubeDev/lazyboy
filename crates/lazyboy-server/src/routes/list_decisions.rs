use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::DecisionDto;

/// `GET /spaces/:id/decisions` -> `Decision[]`.
pub async fn list_decisions(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Result<Json<Vec<DecisionDto>>, ApiError> {
    let rows = repo::decision::list(state.store(), space_id).await?;
    Ok(Json(rows.into_iter().map(DecisionDto::from).collect()))
}
