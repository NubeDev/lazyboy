use axum::extract::{Path, State};
use axum::Json;

use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{RunOutcomeDto, StartRunBody};

/// `POST /spaces/:id/run` body `{prompt}` -> `RunOutcome`
/// (RpcClient.startRun). Reconcile first so a run started in a fresh
/// process after a crash re-drives any in-flight approval before
/// opening new work, matching the CLI's `run` command.
pub async fn start_run(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
    Json(body): Json<StartRunBody>,
) -> Result<Json<RunOutcomeDto>, ApiError> {
    let engine = state.engine().await?;
    engine.reconcile().await?;
    let started = engine
        .start_run(space_id, &body.prompt, &body.prompt)
        .await?;
    Ok(Json(started.outcome.into()))
}
