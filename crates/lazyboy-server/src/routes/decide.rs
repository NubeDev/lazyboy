use axum::extract::{Path, State};
use axum::Json;

use lazyboy_types::domain::Approval;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{DecisionBody, RunOutcomeDto};

/// `POST /approvals/:id/decision` body `{status}` -> `RunOutcome`
/// (RpcClient.decide). Reconcile first so the decision lands even in a
/// fresh process after a crash (the ACP request id is rebuilt by
/// re-driving the loaded session), matching the CLI's `decide`.
pub async fn decide(
    State(state): State<AppState>,
    Path(approval_id): Path<Id<Approval>>,
    Json(body): Json<DecisionBody>,
) -> Result<Json<RunOutcomeDto>, ApiError> {
    let engine = state.engine().await?;
    engine.reconcile().await?;
    let human = state.human().await?;
    match engine
        .resolve_approval(approval_id, body.status, human)
        .await?
    {
        Some(outcome) => Ok(Json(outcome.into())),
        None => Ok(Json(RunOutcomeDto::AlreadyResolved)),
    }
}
