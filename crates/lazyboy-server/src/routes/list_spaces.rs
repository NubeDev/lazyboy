use axum::extract::State;
use axum::Json;

use lazyboy_store::repo;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::SpaceDto;

/// `GET /spaces` -> `Space[]` (RpcClient.listSpaces).
pub async fn list_spaces(State(state): State<AppState>) -> Result<Json<Vec<SpaceDto>>, ApiError> {
    let rows = repo::space::list(state.store()).await?;
    Ok(Json(rows.into_iter().map(SpaceDto::from).collect()))
}
