use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::RunDto;

/// `GET /spaces/:id/runs` -> `AgentRun[]` (RpcClient.listRuns).
pub async fn list_runs(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Result<Json<Vec<RunDto>>, ApiError> {
    let rows = repo::run::list(state.store(), space_id).await?;
    Ok(Json(rows.into_iter().map(RunDto::from).collect()))
}
