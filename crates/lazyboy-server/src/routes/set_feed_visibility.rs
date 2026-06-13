use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Integration;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreatedIdDto, FeedVisibilityBody};

/// `POST /feeds/:integration_id/visibility` body `{space_id,
/// principal_kind, principal_id, mode}`. Sets per (feed, space,
/// principal) access inside a space. Modeled feed visibility, not
/// enforced in the MVP trust gate under R4 (DOCS/WORKFLOWS.md).
pub async fn set_feed_visibility(
    State(state): State<AppState>,
    Path(feed_integration_id): Path<Id<Integration>>,
    Json(body): Json<FeedVisibilityBody>,
) -> Result<Json<CreatedIdDto>, ApiError> {
    let id = repo::membership::set_feed_visibility(
        state.store(),
        feed_integration_id,
        body.space_id,
        &body.principal_kind,
        &body.principal_id,
        &body.mode,
    )
    .await?;
    Ok(Json(CreatedIdDto { id }))
}
