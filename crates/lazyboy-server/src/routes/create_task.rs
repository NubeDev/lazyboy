use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreateTaskBody, TaskDto};

/// `POST /spaces/:id/tasks` body `{title}` -> the created `Task`. The
/// deterministic quick-add behind the `/task` command-bar shortcut: it
/// opens a task directly, with no agent run, so adding a task costs no
/// model turn. Natural-language task creation goes through `start_run`
/// and the agent's `create_task` MCP tool instead.
pub async fn create_task(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
    Json(body): Json<CreateTaskBody>,
) -> Result<Json<TaskDto>, ApiError> {
    let title = body.title.trim();
    if title.is_empty() {
        return Err(ApiError::BadRequest("task title is empty".to_owned()));
    }
    let id = repo::task::create(state.store(), space_id, title, None).await?;
    let rows = repo::task::list(state.store(), space_id).await?;
    let row = rows
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| ApiError::NotFound("task vanished after create".to_owned()))?;
    Ok(Json(row.into()))
}
