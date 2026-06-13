use axum::extract::State;
use axum::Json;

use lazyboy_store::repo;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreateWorkflowBody, WorkflowDto};

/// `POST /workflows` body `{workspace_id, name, trigger_kind,
/// trigger_config_json?, approval_policy, steps_json}` -> the created
/// `Workflow`. Created disabled; `enable` arms its trigger.
pub async fn create_workflow(
    State(state): State<AppState>,
    Json(body): Json<CreateWorkflowBody>,
) -> Result<Json<WorkflowDto>, ApiError> {
    let id = repo::workflow::create(
        state.store(),
        repo::workflow::NewWorkflow {
            workspace_id: body.workspace_id,
            name: &body.name,
            trigger_kind: body.trigger_kind,
            trigger_config_json: body.trigger_config_json.as_deref(),
            approval_policy: body.approval_policy,
            steps_json: &body.steps_json,
        },
    )
    .await?;
    let row = repo::workflow::get(state.store(), id).await?;
    Ok(Json(row.into()))
}
