use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::{Workflow, WorkflowStatus};
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::WorkflowDto;

/// `POST /workflows/:id/enable` -> the updated `Workflow`. Arming the
/// trigger is what makes a workflow an automation (SCOPE.md).
pub async fn enable_workflow(
    State(state): State<AppState>,
    Path(id): Path<Id<Workflow>>,
) -> Result<Json<WorkflowDto>, ApiError> {
    set(state, id, WorkflowStatus::Enabled).await
}

/// `POST /workflows/:id/disable` -> the updated `Workflow`.
pub async fn disable_workflow(
    State(state): State<AppState>,
    Path(id): Path<Id<Workflow>>,
) -> Result<Json<WorkflowDto>, ApiError> {
    set(state, id, WorkflowStatus::Disabled).await
}

async fn set(
    state: AppState,
    id: Id<Workflow>,
    status: WorkflowStatus,
) -> Result<Json<WorkflowDto>, ApiError> {
    repo::workflow::set_status(state.store(), id, status).await?;
    let row = repo::workflow::get(state.store(), id).await?;
    Ok(Json(row.into()))
}
