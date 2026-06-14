use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::MembershipDto;

/// `GET /spaces/:id/members` -> `Membership[]`. The read side of the
/// grant: lets the UI show who holds a role in a space. Modeled, not
/// enforced in the MVP trust gate under R4 (DOCS/WORKFLOWS.md).
pub async fn list_members(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Result<Json<Vec<MembershipDto>>, ApiError> {
    let rows = repo::membership::list_memberships(state.store(), space_id).await?;
    Ok(Json(rows.into_iter().map(MembershipDto::from).collect()))
}
