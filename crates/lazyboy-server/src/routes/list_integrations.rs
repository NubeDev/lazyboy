use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use lazyboy_store::repo;
use lazyboy_types::domain::Workspace;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::IntegrationDto;

#[derive(Deserialize)]
pub struct ListQuery {
    pub workspace_id: Id<Workspace>,
}

/// `GET /integrations?workspace_id=...` -> `Integration[]`.
pub async fn list_integrations(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<IntegrationDto>>, ApiError> {
    let rows = repo::integration::list(state.store(), query.workspace_id).await?;
    Ok(Json(rows.into_iter().map(IntegrationDto::from).collect()))
}
