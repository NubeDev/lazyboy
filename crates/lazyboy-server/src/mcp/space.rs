use axum::http::HeaderMap;
use serde_json::Value;

use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::error::ApiError;

/// The header lazyboy sets on the goose MCP config (`mcpServers[].headers`)
/// to bind a session's tool calls to one space. It rides the transport,
/// not a tool argument, so the model cannot redirect a call at another
/// space — the binding is fixed when lazyboy opens the session.
pub const SPACE_HEADER: &str = "x-lazyboy-space";

/// Resolve the space a tool call targets from [`SPACE_HEADER`]. A missing
/// or unparseable header is a client error, not a panic: an MCP client
/// that does not carry it is misconfigured, not malicious.
pub fn space_from(headers: &HeaderMap) -> Result<Id<Space>, ApiError> {
    let raw = headers
        .get(SPACE_HEADER)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest(format!("missing {SPACE_HEADER} header")))?;
    // `Id<Space>` is `#[serde(transparent)]` over a UUID, so deserialising
    // from the header string reuses uuid's own parser without taking a
    // direct dependency on the crate here.
    serde_json::from_value::<Id<Space>>(Value::String(raw.to_owned()))
        .map_err(|_| ApiError::BadRequest(format!("bad {SPACE_HEADER}: {raw}")))
}
