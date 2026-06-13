// Shared test scaffolding: not every consumer exercises every entry
// point (the CLI test only uses the gated server).
#![allow(dead_code)]

//! An in-process stand-in for `goose serve`, reproducing the verified
//! v1.37.0 ACP contract (`DOCS/GOOSE-ACP.md`): the WS upgrade mints the
//! `acp-connection-id`, `POST /acp` requires it, `initialize` replies on
//! the POST, and `session/new` / `session/prompt` reply `202` with their
//! real results delivered asynchronously over the WS. It exists so the
//! transport can be tested end-to-end without launching the real binary
//! (which the sandbox refuses to run).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use tokio::sync::mpsc;

type Senders = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

#[derive(Clone, Default)]
struct AppState {
    senders: Senders,
    /// When set, a prompt gates on one tool (emits a permission request
    /// and withholds the turn response) instead of completing inline.
    gated: bool,
}

/// A running fake server. `base` is the `http://127.0.0.1:PORT` the
/// transport connects to. Dropping it shuts the server down.
pub struct FakeAcp {
    pub base: String,
    _shutdown: tokio::task::JoinHandle<()>,
}

impl FakeAcp {
    pub async fn start() -> Self {
        Self::start_with(false).await
    }

    /// Start a server whose prompt gates on one tool, exercising the
    /// approval round-trip end to end.
    pub async fn start_gated() -> Self {
        Self::start_with(true).await
    }

    async fn start_with(gated: bool) -> Self {
        let state = AppState {
            gated,
            ..AppState::default()
        };
        let app = Router::new()
            .route("/acp", get(ws_upgrade).post(post_acp))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr: SocketAddr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        FakeAcp {
            base: format!("http://{addr}"),
            _shutdown: handle,
        }
    }
}

async fn ws_upgrade(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    let connection_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::unbounded_channel();
    state
        .senders
        .lock()
        .unwrap()
        .insert(connection_id.clone(), tx);
    // The transport reads acp-connection-id off the 101 response.
    let mut resp = upgrade.on_upgrade(move |socket| pump(socket, rx));
    resp.headers_mut()
        .insert("acp-connection-id", connection_id.parse().unwrap());
    resp
}

/// Forward server-authored frames onto the socket, and react to client
/// frames. The only client->server frame in this contract is the answer
/// to a `session/request_permission`; on receiving it, the gated turn
/// completes (tool result + prompt response), modelling goose resuming
/// the suspended tool call.
async fn pump(socket: WebSocket, mut rx: mpsc::UnboundedReceiver<String>) {
    use futures_util::{SinkExt, StreamExt};
    let (mut sink, mut stream) = socket.split();
    loop {
        tokio::select! {
            outgoing = rx.recv() => match outgoing {
                Some(frame) => {
                    if sink.send(Message::Text(frame.into())).await.is_err() {
                        break;
                    }
                }
                None => break,
            },
            incoming = stream.next() => match incoming {
                Some(Ok(Message::Text(t))) => {
                    for frame in on_client_frame(&t) {
                        if sink.send(Message::Text(frame.to_string().into())).await.is_err() {
                            return;
                        }
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(_)) | None => break,
            },
        }
    }
}

/// The answer to a `session/request_permission` (a JSON-RPC response the
/// client sends) resumes the gated tool: emit its result, then the
/// prompt-turn response that the demux turns into TurnEnded.
///
/// The prompt response must echo the id the client used for its
/// `session/prompt`. The transport numbers ids monotonically per
/// connection (initialize=1, session/new=2, session/prompt=3), so the
/// gated turn's prompt id is 3. (`session/load` on a resume would shift
/// this; the round-trip test does not reload.)
fn on_client_frame(text: &str) -> Vec<serde_json::Value> {
    let Ok(frame) = serde_json::from_str::<serde_json::Value>(text) else {
        return vec![];
    };
    // A permission answer carries an `outcome`; ignore anything else.
    if frame.get("result").and_then(|r| r.get("outcome")).is_none() {
        return vec![];
    }
    vec![
        notif(
            "sess-1",
            serde_json::json!({
                "sessionUpdate": "tool_call_update",
                "title": "developer__shell",
                "content": { "stdout": "a.txt" }
            }),
        ),
        serde_json::json!({
            "jsonrpc": "2.0", "id": 3, "result": { "stopReason": "end_turn" }
        }),
    ]
}

async fn post_acp(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let Some(cid) = headers
        .get("acp-connection-id")
        .and_then(|v| v.to_str().ok())
    else {
        return (StatusCode::BAD_REQUEST, "Acp-Connection-Id header required").into_response();
    };
    let req: serde_json::Value = serde_json::from_str(&body).unwrap();
    let id = req["id"].as_u64().unwrap();
    let method = req["method"].as_str().unwrap();

    match method {
        // initialize replies synchronously on the POST.
        "initialize" => axum::Json(serde_json::json!({
            "jsonrpc": "2.0", "id": id,
            "result": { "protocolVersion": 1,
                "agentCapabilities": { "loadSession": true } }
        }))
        .into_response(),

        // Everything else: 202, real result over the WS.
        _ => {
            let tx = state.senders.lock().unwrap().get(cid).cloned();
            if let Some(tx) = tx {
                for frame in frames_for(method, id, &req, state.gated) {
                    let _ = tx.send(frame.to_string());
                }
            }
            StatusCode::ACCEPTED.into_response()
        }
    }
}

/// The WS frames the real goose would emit for a given POSTed method.
fn frames_for(
    method: &str,
    id: u64,
    req: &serde_json::Value,
    gated: bool,
) -> Vec<serde_json::Value> {
    match method {
        "session/new" => vec![serde_json::json!({
            "jsonrpc": "2.0", "id": id, "result": { "sessionId": "sess-1" }
        })],
        "session/load" => vec![serde_json::json!({
            "jsonrpc": "2.0", "id": id, "result": { "sessionId": "sess-1" }
        })],
        // A gated prompt asks permission and withholds the turn response
        // until the client answers (see on_client_frame); an ungated one
        // streams and completes inline.
        "session/prompt" if gated => {
            let session = req["params"]["sessionId"].as_str().unwrap().to_owned();
            vec![serde_json::json!({
                "jsonrpc": "2.0", "id": 99, "method": "session/request_permission",
                "params": { "sessionId": session, "toolCall": {
                    "title": "developer__shell",
                    "rawInput": { "command": "ls" } } }
            })]
        }
        "session/prompt" => {
            let session = req["params"]["sessionId"].as_str().unwrap().to_owned();
            vec![
                notif(
                    &session,
                    serde_json::json!({
                        "sessionUpdate": "agent_message_chunk",
                        "content": { "type": "text", "text": "working" }
                    }),
                ),
                // Turn boundary: the prompt response carries stopReason.
                serde_json::json!({
                    "jsonrpc": "2.0", "id": id, "result": { "stopReason": "end_turn" }
                }),
            ]
        }
        _ => vec![],
    }
}

fn notif(session: &str, update: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0", "method": "session/update",
        "params": { "sessionId": session, "update": update }
    })
}
