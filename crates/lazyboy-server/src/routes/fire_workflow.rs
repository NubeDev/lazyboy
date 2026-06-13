use axum::extract::{Path, State};
use axum::Json;

use lazyboy_types::domain::Workflow;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{FireWorkflowBody, RunOutcomeDto};

/// `POST /workflows/:id/fire` body `{space_id}` -> `RunOutcome`. Fires a
/// saved workflow into a space under its approval policy (auto-approve
/// runs to completion writing-then-resolving each step's audit row;
/// require-approval parks the first step). Reconcile first so a fresh
/// process clears any in-flight approval before opening new work.
pub async fn fire_workflow(
    State(state): State<AppState>,
    Path(workflow_id): Path<Id<Workflow>>,
    Json(body): Json<FireWorkflowBody>,
) -> Result<Json<RunOutcomeDto>, ApiError> {
    let engine = state.engine().await?;
    engine.reconcile().await?;
    let outcome = engine.run_workflow(workflow_id, body.space_id).await?;
    Ok(Json(outcome.into()))
}
