use axum::extract::State;
use axum::Json;

use lazyboy_store::repo;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreateGroupBody, GroupDto};

/// `POST /groups` body `{workspace_id, name}` -> the created `Group`.
/// Part of the membership model: modeled here, NOT enforced in the MVP
/// trust gate under R4 until promoted (DOCS/WORKFLOWS.md).
pub async fn create_group(
    State(state): State<AppState>,
    Json(body): Json<CreateGroupBody>,
) -> Result<Json<GroupDto>, ApiError> {
    let id = repo::membership::create_group(state.store(), body.workspace_id, &body.name).await?;
    Ok(Json(GroupDto {
        id,
        workspace_id: body.workspace_id,
        name: body.name,
    }))
}
