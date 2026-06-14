use axum::extract::State;
use axum::Json;

use lazyboy_adapters_host::GooseServeClient;

use crate::error::ApiError;
use crate::state::AppState;
use crate::wire::HealthDto;

/// `GET /health` -> goose reachability. Probes by attempting the same
/// connect/initialize handshake a mutating request would, so the status
/// reflects whether real work could run right now, not just whether a
/// socket is open. A failed probe is reported in the body (200 with
/// `goose_reachable: false`), not as an error status: the node itself is
/// healthy, goose is the dependency that is down.
pub async fn health(State(state): State<AppState>) -> Result<Json<HealthDto>, ApiError> {
    let (goose_reachable, goose_detail) = match GooseServeClient::connect(state.goose_url()).await {
        Ok(_) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };
    Ok(Json(HealthDto {
        goose_url: state.goose_url().to_owned(),
        goose_reachable,
        goose_detail,
    }))
}
