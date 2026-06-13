use axum::extract::State;
use axum::Json;

use lazyboy_store::repo;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{CreateIntegrationBody, IntegrationDto};

/// `POST /integrations` body `{workspace_id, provider, account_ref?,
/// secret_ref?, config_json?}` -> the created `Integration`. The body
/// carries only a `secret_ref` (a host secrets-store pointer); a raw
/// secret has no field here (SCOPE.md R5).
pub async fn create_integration(
    State(state): State<AppState>,
    Json(body): Json<CreateIntegrationBody>,
) -> Result<Json<IntegrationDto>, ApiError> {
    let config_json = body.config_json.as_ref().map(ToString::to_string);
    let id = repo::integration::create(
        state.store(),
        repo::integration::NewIntegration {
            workspace_id: body.workspace_id,
            provider: body.provider,
            account_ref: body.account_ref.as_deref(),
            secret_ref: body.secret_ref.as_deref(),
            config_json: config_json.as_deref(),
        },
    )
    .await?;
    let row = repo::integration::get(state.store(), id)
        .await?
        .ok_or_else(|| ApiError::NotFound("integration vanished after create".to_owned()))?;
    Ok(Json(row.into()))
}
