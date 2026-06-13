use axum::extract::{Path, State};
use axum::Json;

use lazyboy_ingress::{self, Bindings};
use lazyboy_store::repo;
use lazyboy_types::domain::Integration;
use lazyboy_types::Id;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::{IngestResultDto, IngressBody};

/// `POST /integrations/:id/ingress` — the webhook sink for MVP. Body is
/// a raw provider payload plus an optional explicit `space_id`. The
/// handler resolves the integration, normalizes the payload through
/// `lazyboy-ingress` for the integration's provider, resolves the bound
/// space (explicit `space_id`, else the integration's `config_json`
/// bindings), then calls `repo::ingress::ingest`, which dedups on
/// `(integration_id, external_id)`. Returns the resulting message id and
/// whether the call was a deduped redelivery.
pub async fn ingress(
    State(state): State<AppState>,
    Path(integration_id): Path<Id<Integration>>,
    Json(body): Json<IngressBody>,
) -> Result<Json<IngestResultDto>, ApiError> {
    let integration = repo::integration::get(state.store(), integration_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("integration".to_owned()))?;

    let event = lazyboy_ingress::normalize(integration.provider, &body.payload)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let space_id = match body.space_id {
        Some(space_id) => space_id,
        None => {
            let bindings = parse_bindings(integration.config_json.as_deref())?;
            lazyboy_ingress::resolve_space(&bindings, &body.payload).ok_or_else(|| {
                ApiError::BadRequest(
                    "no space_id given and no config_json binding matched the payload".to_owned(),
                )
            })?
        }
    };

    let author = state.ingress_author().await?;
    let payload_json = body.payload.to_string();
    let outcome = repo::ingress::ingest(
        state.store(),
        repo::ingress::NewIngress {
            integration_id,
            space_id,
            author,
            external_id: &event.external_id,
            kind: &event.kind,
            payload_json: &payload_json,
            body: &event.body,
        },
    )
    .await?;

    Ok(Json(IngestResultDto {
        message_id: outcome.message_id,
        deduped: outcome.deduped,
    }))
}

fn parse_bindings(config_json: Option<&str>) -> Result<Bindings, ApiError> {
    match config_json {
        None => Ok(Bindings::default()),
        Some(raw) => serde_json::from_str(raw)
            .map_err(|e| ApiError::BadRequest(format!("integration config_json invalid: {e}"))),
    }
}
