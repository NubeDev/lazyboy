use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use lazyboy_store::repo;
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::WorkflowDto;

#[derive(Deserialize)]
pub struct ListQuery {
    pub workspace_id: Id<Workspace>,
}

/// `GET /workflows?workspace_id=...` -> `Workflow[]`.
pub async fn list_workflows(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<WorkflowDto>>, ApiError> {
    let rows = repo::workflow::list(state.store(), query.workspace_id).await?;
    Ok(Json(rows.into_iter().map(WorkflowDto::from).collect()))
}
