use axum::extract::State;
use axum::Json;

use lazyboy_adapters_host::PROVIDERS;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{GooseConfigDto, GooseProviderDto, SetGooseConfigBody};

/// `GET /goose/providers` -> the selectable providers, each flagged with
/// whether a key is already stored (never the key itself, SCOPE.md R5).
pub async fn list_goose_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<GooseProviderDto>>, ApiError> {
    let store = state.goose_config();
    let mut out = Vec::with_capacity(PROVIDERS.len());
    for p in PROVIDERS {
        out.push(GooseProviderDto {
            id: p.id.to_owned(),
            display_name: p.display_name.to_owned(),
            requires_key: p.requires_key,
            key_set: store.has_key(p.key_env)?,
            models: p.models.iter().map(|m| (*m).to_owned()).collect(),
        });
    }
    Ok(Json(out))
}

/// `GET /goose/config` -> the current selection and whether goose is
/// running under it.
pub async fn get_goose_config(
    State(state): State<AppState>,
) -> Result<Json<GooseConfigDto>, ApiError> {
    let selection = state.goose_config().selection()?;
    Ok(Json(GooseConfigDto {
        provider: selection.provider,
        model: selection.model,
        running: state.goose().running().await,
    }))
}

/// `POST /goose/config` -> persist the selection (and key when given),
/// then relaunch goose so the new provider takes effect. Returns the
/// applied config including live running state. A relaunch failure (a bad
/// key, a provider goose rejects) surfaces as a 502 so the UI can show
/// why, with the selection already saved.
pub async fn set_goose_config(
    State(state): State<AppState>,
    Json(body): Json<SetGooseConfigBody>,
) -> Result<Json<GooseConfigDto>, ApiError> {
    let selection = state.goose_config().save(
        &body.provider,
        body.model.as_deref(),
        body.api_key.as_deref(),
    )?;
    state.goose().restart().await?;
    Ok(Json(GooseConfigDto {
        provider: selection.provider,
        model: selection.model,
        running: state.goose().running().await,
    }))
}
