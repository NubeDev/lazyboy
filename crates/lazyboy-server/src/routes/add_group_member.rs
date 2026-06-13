use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Group;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::AddMemberBody;

/// `POST /groups/:id/members` body `{identity_id}`. Modeled membership,
/// not enforced in the MVP trust gate under R4 (DOCS/WORKFLOWS.md).
pub async fn add_group_member(
    State(state): State<AppState>,
    Path(group_id): Path<Id<Group>>,
    Json(body): Json<AddMemberBody>,
) -> Result<StatusCode, ApiError> {
    repo::membership::add_member(state.store(), group_id, body.identity_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
