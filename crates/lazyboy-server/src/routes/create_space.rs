use axum::extract::State;
use axum::Json;

use lazyboy_store::repo;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreateSpaceBody, SpaceDto};

/// `POST /spaces` body `{slug, title}` -> the created `Space`. The
/// workspace is resolved server-side (single trust boundary, SCOPE R5).
/// A slug already taken in the workspace is a 400, not a 500, since it
/// is a client-correctable conflict.
pub async fn create_space(
    State(state): State<AppState>,
    Json(body): Json<CreateSpaceBody>,
) -> Result<Json<SpaceDto>, ApiError> {
    let slug = body.slug.trim();
    if slug.is_empty() {
        return Err(ApiError::BadRequest("slug must not be empty".to_owned()));
    }
    let workspace_id = repo::workspace::current(state.store()).await?;
    let id = repo::bootstrap::create_space(state.store(), workspace_id, slug, body.title.trim())
        .await
        .map_err(|e| {
            if e.is_unique_violation() {
                ApiError::BadRequest(format!("slug '{slug}' already in use"))
            } else {
                ApiError::from(e)
            }
        })?;
    let rows = repo::space::list(state.store()).await?;
    let row = rows
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| ApiError::NotFound("space vanished after create".to_owned()))?;
    Ok(Json(row.into()))
}
