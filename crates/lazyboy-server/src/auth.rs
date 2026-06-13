use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::error::ApiError;
use crate::state::AppState;

/// Single-tenant bearer gate (SCOPE.md R4): one token authorises the
/// browser, CLI, and future mobile clients identically. If the server
/// was started without a token the gate is open, which is the dev
/// default — there is no per-route or per-client scoping to add.
pub async fn require_bearer(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let expected = match state.token() {
        None => return Ok(next.run(request).await),
        Some(t) => t,
    };

    let presented = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match presented {
        Some(token) if token == expected => Ok(next.run(request).await),
        _ => Err(ApiError::Unauthorized),
    }
}
