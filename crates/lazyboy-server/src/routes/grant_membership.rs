use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreatedIdDto, GrantMembershipBody};

/// `POST /spaces/:id/members` body `{principal_kind, principal_id,
/// role}`. Grants a user/group a role in a space. Modeled membership,
/// not enforced in the MVP trust gate under R4 (DOCS/WORKFLOWS.md).
pub async fn grant_membership(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
    Json(body): Json<GrantMembershipBody>,
) -> Result<Json<CreatedIdDto>, ApiError> {
    let id = repo::membership::grant_membership(
        state.store(),
        space_id,
        &body.principal_kind,
        &body.principal_id,
        &body.role,
    )
    .await?;
    Ok(Json(CreatedIdDto { id }))
}
