use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{DecisionDto, RecordDecisionBody};

/// `POST /spaces/:id/decisions` body `{summary, message_id?,
/// decided_by_identity_id?}` -> the recorded `Decision`.
pub async fn record_decision(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
    Json(body): Json<RecordDecisionBody>,
) -> Result<Json<DecisionDto>, ApiError> {
    let id = repo::decision::record(
        state.store(),
        repo::decision::NewDecision {
            space_id,
            message_id: body.message_id,
            summary: &body.summary,
            decided_by_identity_id: body.decided_by_identity_id,
        },
    )
    .await?;
    let rows = repo::decision::list(state.store(), space_id).await?;
    let row = rows
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| ApiError::NotFound("decision vanished after record".to_owned()))?;
    Ok(Json(row.into()))
}
