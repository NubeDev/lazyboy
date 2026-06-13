use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::TaskDto;

/// `GET /spaces/:id/tasks` -> `Task[]` (RpcClient.listTasks).
pub async fn list_tasks(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Result<Json<Vec<TaskDto>>, ApiError> {
    let rows = repo::task::list(state.store(), space_id).await?;
    Ok(Json(rows.into_iter().map(TaskDto::from).collect()))
}
