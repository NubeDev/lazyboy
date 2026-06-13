use serde::Deserialize;

use lazyboy_bridge::{PermissionRequest, ToolCall, Update};

/// A frame off the `GET /acp` WebSocket. Goose multiplexes three things
/// onto this socket: JSON-RPC *responses* to our POSTed requests (keyed
/// by the `id` we sent), agent-initiated `session/update` *notifications*,
/// and agent-initiated `session/request_permission` *requests*. We only
/// model the fields Lazyboy reads; unknown variants are dropped upstream.
#[derive(Debug, Deserialize)]
pub(crate) struct Frame {
    /// Present on a response to one of our calls; absent on agent->client
    /// notifications. `session/request_permission` is a request *from*
    /// the agent and also carries an id, distinguished by `method`.
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

/// What a decoded frame means to the demux loop.
#[derive(Debug, PartialEq)]
pub(crate) enum Decoded {
    /// A reply to the POST we sent with this JSON-RPC id.
    Response {
        id: u64,
        result: Result<serde_json::Value, String>,
    },
    /// An update belonging to a session, to be queued for `next_update`.
    Update { session: String, update: Update },
    /// A permission request: an agent->client JSON-RPC request that the
    /// driver answers out-of-band once a human resolves the approval.
    Permission { session: String, update: Update },
    /// A frame we do not model (telemetry, mode lists, commands). Ignored.
    Ignore,
}

impl Frame {
    pub(crate) fn decode(self) -> Decoded {
        match self.method.as_deref() {
            Some("session/update") => decode_session_update(self.params),
            Some("session/request_permission") => decode_permission(self.id, self.params),
            // No method + an id => a response to one of our requests.
            None => match self.id {
                Some(id) => Decoded::Response {
                    id,
                    result: match self.error {
                        Some(e) => Err(e.to_string()),
                        None => Ok(self.result.unwrap_or(serde_json::Value::Null)),
                    },
                },
                None => Decoded::Ignore,
            },
            _ => Decoded::Ignore,
        }
    }
}

fn session_id(params: &serde_json::Value) -> Option<String> {
    params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

/// Map a `session/update` notification onto an `Update`, or `Ignore` for
/// the update kinds Lazyboy does not import (mode changes, command lists).
fn decode_session_update(params: Option<serde_json::Value>) -> Decoded {
    let Some(params) = params else {
        return Decoded::Ignore;
    };
    let Some(session) = session_id(&params) else {
        return Decoded::Ignore;
    };
    let update = &params["update"];
    match update.get("sessionUpdate").and_then(|v| v.as_str()) {
        Some("agent_message_chunk") | Some("agent_message") => {
            let text = update
                .get("content")
                .and_then(|c| c.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_owned();
            Decoded::Update {
                session,
                update: Update::AgentMessage { text },
            }
        }
        Some("tool_call_update") | Some("tool_call") => {
            let tool_name = update
                .get("title")
                .or_else(|| update.get("toolName"))
                .and_then(|t| t.as_str())
                .unwrap_or_default()
                .to_owned();
            let output_json = update
                .get("content")
                .map(|c| c.to_string())
                .unwrap_or_else(|| "null".to_owned());
            Decoded::Update {
                session,
                update: Update::ToolResult {
                    tool_name,
                    output_json,
                },
            }
        }
        _ => Decoded::Ignore,
    }
}

/// Map a `session/request_permission` request onto a `PermissionRequested`
/// update. The request `id` becomes the ACP request id echoed back with
/// the decision (`answer_permission`).
fn decode_permission(id: Option<u64>, params: Option<serde_json::Value>) -> Decoded {
    let (Some(id), Some(params)) = (id, params) else {
        return Decoded::Ignore;
    };
    let Some(session) = session_id(&params) else {
        return Decoded::Ignore;
    };
    let call = &params["toolCall"];
    let name = call
        .get("title")
        .or_else(|| call.get("toolName"))
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_owned();
    let input_json = call
        .get("rawInput")
        .or_else(|| call.get("input"))
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_owned());
    Decoded::Permission {
        session,
        update: Update::PermissionRequested(PermissionRequest {
            request_id: id.to_string(),
            tool: ToolCall { name, input_json },
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(json: serde_json::Value) -> Decoded {
        serde_json::from_value::<Frame>(json).unwrap().decode()
    }

    #[test]
    fn response_without_method_is_a_response() {
        let d = decode(serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "result": { "sessionId": "20260613_2" }
        }));
        assert!(matches!(
            d,
            Decoded::Response {
                id: 2,
                result: Ok(_)
            }
        ));
    }

    #[test]
    fn jsonrpc_error_response_carries_the_error() {
        let d = decode(serde_json::json!({
            "jsonrpc": "2.0", "id": 5, "error": { "code": -32000, "message": "boom" }
        }));
        match d {
            Decoded::Response {
                id: 5,
                result: Err(e),
            } => assert!(e.contains("boom")),
            other => panic!("expected error response, got {other:?}"),
        }
    }

    #[test]
    fn agent_message_chunk_becomes_an_update() {
        let d = decode(serde_json::json!({
            "jsonrpc": "2.0", "method": "session/update",
            "params": { "sessionId": "s1", "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": "hi" } } }
        }));
        assert_eq!(
            d,
            Decoded::Update {
                session: "s1".into(),
                update: Update::AgentMessage { text: "hi".into() }
            }
        );
    }

    #[test]
    fn request_permission_carries_tool_and_request_id() {
        let d = decode(serde_json::json!({
            "jsonrpc": "2.0", "id": 7, "method": "session/request_permission",
            "params": { "sessionId": "s1", "toolCall": {
                "title": "developer__text_editor",
                "rawInput": { "path": "./x", "command": "write" } } }
        }));
        match d {
            Decoded::Permission {
                session,
                update: Update::PermissionRequested(p),
            } => {
                assert_eq!(session, "s1");
                assert_eq!(p.request_id, "7");
                assert_eq!(p.tool.name, "developer__text_editor");
                assert!(p.tool.input_json.contains("\"command\":\"write\""));
            }
            other => panic!("expected permission, got {other:?}"),
        }
    }

    #[test]
    fn unmodelled_update_kinds_are_ignored() {
        let d = decode(serde_json::json!({
            "jsonrpc": "2.0", "method": "session/update",
            "params": { "sessionId": "s1", "update": {
                "sessionUpdate": "available_commands_update", "availableCommands": [] } }
        }));
        assert_eq!(d, Decoded::Ignore);
    }
}
