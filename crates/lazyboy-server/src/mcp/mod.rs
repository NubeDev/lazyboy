//! A minimal MCP server (Model Context Protocol, streamable-HTTP) that
//! exposes lazyboy's own domain to the rented goose agent. This is the
//! keystone that lets "talk become tasks": without a tool that touches
//! lazyboy, the agent can only run goose's built-in shell/file tools and
//! can never read or change a space. goose connects here via the
//! `mcpServers` entry lazyboy sets on `session/new` (see
//! `lazyboy-adapters-host`), carrying the space id in a header.
//!
//! The protocol surface is intentionally tiny — `initialize`,
//! `tools/list`, `tools/call`, `ping` — hand-rolled as JSON-RPC over a
//! single POST, matching how this crate hand-rolls the goose ACP wire
//! rather than pulling a protocol library. The server is stateless: it
//! mints no MCP session id, so every request is self-contained and the
//! space binding rides the transport header on each call.

mod space;
mod tools;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::state::AppState;

/// The newest MCP protocol revision lazyboy speaks; used when a client
/// omits `protocolVersion` on `initialize`. When the client names one,
/// it is echoed back so version negotiation never downgrades a client
/// that is ahead of this default.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// `POST /mcp` — the single JSON-RPC endpoint. A request (carries `id`)
/// gets a JSON-RPC response; a notification (no `id`) is acknowledged
/// with `202` and an empty body, per JSON-RPC 2.0.
pub async fn mcp(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    let req: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return rpc_error(Value::Null, -32700, &format!("parse error: {e}")),
    };

    let method = req.get("method").and_then(Value::as_str).unwrap_or_default();
    let id = req.get("id").cloned();

    // No id ⇒ a notification (e.g. `notifications/initialized`): ack and
    // return nothing, never a JSON-RPC response.
    let Some(id) = id else {
        return StatusCode::ACCEPTED.into_response();
    };

    match method {
        "initialize" => {
            let version = req
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION)
                .to_owned();
            rpc_ok(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "lazyboy", "version": env!("CARGO_PKG_VERSION") },
                }),
            )
        }
        "ping" => rpc_ok(id, json!({})),
        "tools/list" => rpc_ok(id, json!({ "tools": tools::definitions() })),
        "tools/call" => tools_call(&state, &headers, id, req.get("params")).await,
        other => rpc_error(id, -32601, &format!("method not found: {other}")),
    }
}

async fn tools_call(
    state: &AppState,
    headers: &HeaderMap,
    id: Value,
    params: Option<&Value>,
) -> Response {
    let space_id = match space::space_from(headers) {
        Ok(s) => s,
        // A bad space binding is a protocol-level failure of the call,
        // not a tool-execution result, so it surfaces as a JSON-RPC error.
        Err(e) => return rpc_error(id, -32602, &error_text(e)),
    };
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let empty = json!({});
    let args = params.and_then(|p| p.get("arguments")).unwrap_or(&empty);

    match tools::call(state, space_id, name, args).await {
        Ok(text) => rpc_ok(
            id,
            json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        ),
        // A tool that failed returns an MCP tool result with `isError`,
        // not a JSON-RPC error: the agent sees the failure as tool output
        // it can react to, which is how MCP models recoverable failures.
        Err(e) => rpc_ok(
            id,
            json!({ "content": [{ "type": "text", "text": error_text(e) }], "isError": true }),
        ),
    }
}

fn rpc_ok(id: Value, result: Value) -> Response {
    Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })).into_response()
}

fn rpc_error(id: Value, code: i64, message: &str) -> Response {
    Json(json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }))
        .into_response()
}

/// Render an `ApiError` to the short text an MCP result carries. The HTTP
/// status the error would otherwise produce is irrelevant here — MCP
/// transports the failure inside a `200` JSON-RPC envelope.
fn error_text(e: crate::error::ApiError) -> String {
    use crate::error::ApiError::*;
    match e {
        Unauthorized => "unauthorized".to_owned(),
        BadRequest(m) | NotFound(m) => m,
        Store(e) => e.to_string(),
        Core(e) => e.to_string(),
        Bridge(e) => e.to_string(),
    }
}
