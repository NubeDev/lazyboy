use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::tungstenite::Message;

use lazyboy_bridge::{BridgeError, Update};

use crate::wire::{Decoded, Frame};

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Result<serde_json::Value, String>>>>>;
type Queues = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<Update>>>>;
/// Outbound WS frames. The reader owns the split sink, so writes (pong,
/// and our responses to agent-initiated requests) funnel through here.
type Outbound = mpsc::UnboundedSender<Message>;
/// JSON-RPC ids of in-flight `session/prompt` requests, mapped to their
/// session. The response to one of these is the turn boundary, not a
/// value any caller awaits, so the demux converts it to a `TurnEnded`
/// update on the session's queue rather than resolving a oneshot.
type PromptIds = Arc<Mutex<HashMap<u64, String>>>;

/// One live connection to `goose serve`: the WebSocket (which owns the
/// connection identity) plus the HTTP base used for `POST /acp`. All
/// JSON-RPC replies and agent-initiated frames arrive on the WS and are
/// demuxed by a background reader into per-request oneshots and
/// per-session update channels.
pub(crate) struct Connection {
    http: reqwest::Client,
    base: String,
    connection_id: String,
    next_id: AtomicU64,
    pending: Pending,
    queues: Queues,
    prompt_ids: PromptIds,
    outbound: Outbound,
}

impl Connection {
    /// Open the WebSocket, capture `acp-connection-id` off the upgrade
    /// response, and start the reader task. The WS must exist before any
    /// POST: goose rejects POSTs whose `acp-connection-id` it does not
    /// recognise (`GOOSE-ACP.md` "Connection model").
    pub(crate) async fn open(base: &str) -> Result<Self, BridgeError> {
        let ws_url = format!("{}/acp", base.replace("http", "ws"));
        let req = Request::builder()
            .uri(&ws_url)
            .header("Host", host_of(base))
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
            .body(())
            .map_err(|e| BridgeError::Transport(format!("ws request: {e}")))?;

        let (ws, response) = tokio_tungstenite::connect_async(req)
            .await
            .map_err(|e| BridgeError::Transport(format!("ws connect: {e}")))?;

        let connection_id = response
            .headers()
            .get("acp-connection-id")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| BridgeError::Transport("ws upgrade missing acp-connection-id".into()))?
            .to_owned();

        let pending: Pending = Arc::default();
        let queues: Queues = Arc::default();
        let prompt_ids: PromptIds = Arc::default();
        let (outbound, out_rx) = mpsc::unbounded_channel();
        spawn_reader(
            ws,
            pending.clone(),
            queues.clone(),
            prompt_ids.clone(),
            out_rx,
        );

        Ok(Self {
            http: reqwest::Client::new(),
            base: base.to_owned(),
            connection_id,
            next_id: AtomicU64::new(1),
            pending,
            queues,
            prompt_ids,
            outbound,
        })
    }

    /// Send a JSON-RPC request and await its reply off the WebSocket.
    /// `session/new` and friends acknowledge the POST with `202` and an
    /// empty body; the real result arrives on the WS keyed by this id, so
    /// we register the oneshot before POSTing to avoid a race.
    pub(crate) async fn request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, BridgeError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);

        // `initialize` answers synchronously in the POST body (`200`);
        // `session/new` and `session/load` answer `202` empty and deliver
        // the JSON-RPC result over the WS (`GOOSE-ACP.md` "Connection
        // model"). If the POST carried the result inline, use it and drop
        // the WS oneshot; otherwise await the WS.
        match self.post(id, method, params).await {
            Ok(Some(result)) => {
                self.pending.lock().unwrap().remove(&id);
                jsonrpc_result(result)
            }
            Ok(None) => rx
                .await
                .map_err(|_| BridgeError::Transport(format!("{method}: connection closed")))?
                .map_err(BridgeError::Transport),
            Err(e) => {
                self.pending.lock().unwrap().remove(&id);
                Err(e)
            }
        }
    }

    /// Fire a `session/prompt` and return once goose acknowledges the
    /// POST. The turn-end response is not awaited here; the demux maps it
    /// to a `TurnEnded` on the session queue (see `PromptIds`), so the
    /// driver can pull streamed updates and approval gates in between.
    pub(crate) async fn prompt(
        &self,
        session: &str,
        params: serde_json::Value,
    ) -> Result<(), BridgeError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.prompt_ids
            .lock()
            .unwrap()
            .insert(id, session.to_owned());
        // A prompt is acknowledged with `202`; its turn-boundary response
        // arrives over the WS and the demux converts it to TurnEnded. We
        // do not consume any inline body here.
        if let Err(e) = self.post(id, "session/prompt", params).await {
            self.prompt_ids.lock().unwrap().remove(&id);
            return Err(e);
        }
        Ok(())
    }

    /// POST one JSON-RPC request. Returns the inline result value when the
    /// server answered synchronously (a `200` with a non-empty body), or
    /// `None` when it acknowledged with `202` and will answer over the WS.
    async fn post(
        &self,
        id: u64,
        method: &str,
        params: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, BridgeError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params,
        });
        let resp = self
            .http
            .post(format!("{}/acp", self.base))
            .header("acp-connection-id", &self.connection_id)
            .json(&body)
            .send()
            .await
            .map_err(|e| BridgeError::Transport(format!("{method} post: {e}")))?;
        if !resp.status().is_success() {
            return Err(BridgeError::Transport(format!(
                "{method} post: status {}",
                resp.status()
            )));
        }
        let text = resp
            .text()
            .await
            .map_err(|e| BridgeError::Transport(format!("{method} body: {e}")))?;
        if text.trim().is_empty() {
            return Ok(None);
        }
        let frame: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| BridgeError::Transport(format!("{method} body parse: {e}")))?;
        Ok(Some(frame))
    }

    /// Register interest in a session's updates, returning the receiving
    /// end of the channel the reader pushes onto. Called once per session
    /// (on new/load) before the driver starts pulling.
    pub(crate) fn subscribe(&self, session: &str) -> mpsc::UnboundedReceiver<Update> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.queues.lock().unwrap().insert(session.to_owned(), tx);
        rx
    }

    /// Reply to an agent-initiated JSON-RPC request (e.g.
    /// `session/request_permission`) by sending a response frame with the
    /// matching id back over the WebSocket. Unlike `request`, this expects
    /// no reply — goose consumes it to unblock the gated tool.
    pub(crate) async fn respond(
        &self,
        id: u64,
        result: serde_json::Value,
    ) -> Result<(), BridgeError> {
        let frame = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result });
        self.outbound
            .send(Message::Text(frame.to_string().into()))
            .map_err(|_| BridgeError::Transport("ws closed; cannot answer permission".into()))
    }
}

fn host_of(base: &str) -> String {
    base.trim_start_matches("http://")
        .trim_start_matches("https://")
        .to_owned()
}

fn spawn_reader<S>(
    ws: S,
    pending: Pending,
    queues: Queues,
    prompt_ids: PromptIds,
    mut out_rx: mpsc::UnboundedReceiver<Message>,
) where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + SinkExt<Message>
        + Send
        + Unpin
        + 'static,
{
    tokio::spawn(async move {
        let (mut sink, mut stream) = ws.split();
        loop {
            tokio::select! {
                outgoing = out_rx.recv() => match outgoing {
                    Some(msg) => {
                        if sink.send(msg).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                },
                incoming = stream.next() => {
                    let Some(msg) = incoming else { break };
                    let text = match msg {
                        Ok(Message::Text(t)) => t.to_string(),
                        Ok(Message::Ping(p)) => {
                            let _ = sink.send(Message::Pong(p)).await;
                            continue;
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        Ok(_) => continue,
                    };
                    let Ok(frame) = serde_json::from_str::<Frame>(&text) else {
                        continue;
                    };
                    dispatch(frame.decode(), &pending, &queues, &prompt_ids);
                }
            }
        }
        drain_on_close(&pending, &queues, &prompt_ids);
    });
}

/// Fail every outstanding request and close every in-flight turn when the
/// socket drops, so callers surface a transport error or a non-clean end
/// instead of hanging on a reply that will never come. Reconcile re-drives
/// the closed runs from the durable rows.
fn drain_on_close(pending: &Pending, queues: &Queues, prompt_ids: &PromptIds) {
    for (_, tx) in pending.lock().unwrap().drain() {
        let _ = tx.send(Err("websocket closed".into()));
    }
    let queues = queues.lock().unwrap();
    for (_, session) in prompt_ids.lock().unwrap().drain() {
        if let Some(tx) = queues.get(&session) {
            let _ = tx.send(Update::TurnEnded { stopped: false });
        }
    }
}

fn dispatch(decoded: Decoded, pending: &Pending, queues: &Queues, prompt_ids: &PromptIds) {
    match decoded {
        Decoded::Response { id, result } => {
            // A response to a `session/prompt` is the turn boundary: route
            // it onto the session queue as `TurnEnded` rather than to a
            // waiting oneshot (prompt() does not await it).
            if let Some(session) = prompt_ids.lock().unwrap().remove(&id) {
                let stopped = matches!(
                    result.as_ref().ok().and_then(stop_reason).as_deref(),
                    Some("end_turn") | Some("max_tokens")
                );
                if let Some(tx) = queues.lock().unwrap().get(&session) {
                    let _ = tx.send(Update::TurnEnded { stopped });
                }
                return;
            }
            if let Some(tx) = pending.lock().unwrap().remove(&id) {
                let _ = tx.send(result);
            }
        }
        Decoded::Update { session, update } | Decoded::Permission { session, update } => {
            if let Some(tx) = queues.lock().unwrap().get(&session) {
                let _ = tx.send(update);
            }
        }
        Decoded::Ignore => {}
    }
}

/// Pull the `result` value out of a JSON-RPC response frame, or map its
/// `error` to a transport error.
fn jsonrpc_result(frame: serde_json::Value) -> Result<serde_json::Value, BridgeError> {
    if let Some(err) = frame.get("error") {
        return Err(BridgeError::Transport(err.to_string()));
    }
    Ok(frame
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

fn stop_reason(result: &serde_json::Value) -> Option<String> {
    result
        .get("stopReason")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazyboy_bridge::{PermissionRequest, ToolCall};

    fn maps() -> (Pending, Queues, PromptIds) {
        (Arc::default(), Arc::default(), Arc::default())
    }

    fn subscribe(queues: &Queues, session: &str) -> mpsc::UnboundedReceiver<Update> {
        let (tx, rx) = mpsc::unbounded_channel();
        queues.lock().unwrap().insert(session.to_owned(), tx);
        rx
    }

    #[test]
    fn prompt_response_becomes_turn_ended_not_a_oneshot_reply() {
        let (pending, queues, prompt_ids) = maps();
        let mut rx = subscribe(&queues, "s1");
        // The id was registered by prompt() as an in-flight turn.
        prompt_ids.lock().unwrap().insert(9, "s1".into());

        dispatch(
            Decoded::Response {
                id: 9,
                result: Ok(serde_json::json!({ "stopReason": "end_turn" })),
            },
            &pending,
            &queues,
            &prompt_ids,
        );

        assert_eq!(rx.try_recv().unwrap(), Update::TurnEnded { stopped: true });
        assert!(
            prompt_ids.lock().unwrap().is_empty(),
            "the prompt id is consumed"
        );
    }

    #[test]
    fn a_non_end_stop_reason_is_a_failed_turn() {
        let (pending, queues, prompt_ids) = maps();
        let mut rx = subscribe(&queues, "s1");
        prompt_ids.lock().unwrap().insert(3, "s1".into());

        dispatch(
            Decoded::Response {
                id: 3,
                result: Ok(serde_json::json!({ "stopReason": "refusal" })),
            },
            &pending,
            &queues,
            &prompt_ids,
        );

        assert_eq!(rx.try_recv().unwrap(), Update::TurnEnded { stopped: false });
    }

    #[tokio::test]
    async fn a_non_prompt_response_resolves_its_oneshot() {
        let (pending, queues, prompt_ids) = maps();
        let (tx, rx) = oneshot::channel();
        pending.lock().unwrap().insert(2, tx);

        dispatch(
            Decoded::Response {
                id: 2,
                result: Ok(serde_json::json!({ "sessionId": "s1" })),
            },
            &pending,
            &queues,
            &prompt_ids,
        );

        let got = rx.await.unwrap().unwrap();
        assert_eq!(got["sessionId"], "s1");
    }

    #[test]
    fn permission_and_update_route_to_the_session_queue_in_order() {
        let (pending, queues, prompt_ids) = maps();
        let mut rx = subscribe(&queues, "s1");

        dispatch(
            Decoded::Update {
                session: "s1".into(),
                update: Update::AgentMessage { text: "hi".into() },
            },
            &pending,
            &queues,
            &prompt_ids,
        );
        dispatch(
            Decoded::Permission {
                session: "s1".into(),
                update: Update::PermissionRequested(PermissionRequest {
                    request_id: "7".into(),
                    tool: ToolCall {
                        name: "shell".into(),
                        input_json: "{}".into(),
                    },
                }),
            },
            &pending,
            &queues,
            &prompt_ids,
        );

        assert_eq!(
            rx.try_recv().unwrap(),
            Update::AgentMessage { text: "hi".into() }
        );
        assert!(matches!(
            rx.try_recv().unwrap(),
            Update::PermissionRequested(_)
        ));
    }

    #[tokio::test]
    async fn dropping_the_socket_fails_requests_and_closes_turns() {
        let (pending, queues, prompt_ids) = maps();
        let (tx, rx) = oneshot::channel();
        pending.lock().unwrap().insert(1, tx);
        let mut updates = subscribe(&queues, "s1");
        prompt_ids.lock().unwrap().insert(5, "s1".into());

        drain_on_close(&pending, &queues, &prompt_ids);

        assert!(rx.await.unwrap().is_err(), "outstanding request failed");
        assert_eq!(
            updates.try_recv().unwrap(),
            Update::TurnEnded { stopped: false },
            "in-flight turn closed as failed so the driver does not hang"
        );
    }
}
