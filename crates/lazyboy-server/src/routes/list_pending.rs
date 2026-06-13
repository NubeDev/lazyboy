use axum::extract::{Path, State};
use axum::Json;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::ApprovalDto;

/// `GET /spaces/:id/pending` -> `Approval[]` (RpcClient.listPending).
pub async fn list_pending(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Result<Json<Vec<ApprovalDto>>, ApiError> {
    let rows = repo::approval::list_pending(state.store(), space_id).await?;
    Ok(Json(rows.into_iter().map(ApprovalDto::from).collect()))
}
