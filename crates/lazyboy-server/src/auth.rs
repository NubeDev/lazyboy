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

    let header_token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::to_owned);

    // The SSE subscribe endpoint is reached from the browser through
    // `EventSource`, which cannot set an Authorization header; its only
    // channel for the bearer is the query string. Accept a `token` query
    // param as equivalent so the single token (R4) still gates the
    // stream. The header path is preferred for every other call.
    let presented = header_token.or_else(|| token_from_query(request.uri().query()));

    match presented.as_deref() {
        Some(token) if token == expected => Ok(next.run(request).await),
        _ => Err(ApiError::Unauthorized),
    }
}

fn token_from_query(query: Option<&str>) -> Option<String> {
    query?.split('&').find_map(|pair| {
        pair.strip_prefix("token=")
            .map(|v| v.replace('+', " "))
            .map(|v| percent_decode(&v))
    })
}

/// Minimal percent-decode for the token query param. A bearer token is
/// typically url-safe, but `+`/`%XX` can appear; decoding here avoids a
/// new dependency for one short string.
fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next();
            let lo = bytes.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(h), Some(l)) = ((h as char).to_digit(16), (l as char).to_digit(16)) {
                    out.push((h as u8 * 16 + l as u8) as char);
                    continue;
                }
            }
            out.push('%');
        } else {
            out.push(b as char);
        }
    }
    out
}
