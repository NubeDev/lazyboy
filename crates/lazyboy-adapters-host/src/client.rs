use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use tokio::sync::mpsc;

use lazyboy_bridge::{BridgeError, Decision, GooseClient, SessionId, Update};

use crate::conn::Connection;

/// A live `GooseClient` over `goose serve` (ACP-over-HTTP, v1.37.0).
/// One `Connection` (one WebSocket) backs all sessions; each session has
/// an update receiver drained by `next_update`. This is the production
/// implementation of the seam `lazyboy-bridge` defines and tests against
/// `FakeGoose`.
pub struct GooseServeClient {
    conn: Connection,
    inboxes: Mutex<HashMap<String, mpsc::UnboundedReceiver<Update>>>,
    /// Absolute working directory handed to goose on `session/new` and
    /// `session/load`. Goose rejects a relative `cwd` (`cwd must be an
    /// absolute path`), so it is canonicalized once at connect time.
    cwd: String,
    /// The lazyboy MCP server goose connects back to, or `None` to open
    /// sessions with no lazyboy tools (the CLI/Tauri shells, and tests).
    /// When set, every session carries it in `mcpServers` so the agent
    /// can read and act on the space it is scoped to.
    lazyboy_mcp: Option<LazyboyMcp>,
}

/// The lazyboy MCP endpoint and bearer goose is told to reach. `url` is
/// the server's own `/mcp` route; `token` is the single-tenant bearer
/// (SCOPE.md R4) goose must present to pass the auth gate, or `None` in
/// the auth-disabled dev mode.
#[derive(Clone)]
struct LazyboyMcp {
    url: String,
    token: Option<String>,
}

impl GooseServeClient {
    /// Connect to a running `goose serve` at `base` (e.g.
    /// `http://127.0.0.1:3284`), defaulting the session working directory
    /// to the current process directory. See [`Self::connect_in`].
    pub async fn connect(base: &str) -> Result<Self, BridgeError> {
        let cwd = std::env::current_dir()
            .map_err(|e| BridgeError::Transport(format!("cannot read current dir: {e}")))?;
        Self::connect_in(base, &cwd).await
    }

    /// Connect and perform the ACP `initialize` handshake, scoping
    /// sessions to `cwd`. `loadSession: true` in the response is required
    /// for the crash-resume reconcile to work; we reject a server without
    /// it rather than discover the gap at recovery time. `cwd` is
    /// canonicalized to an absolute path because goose rejects a relative
    /// one.
    pub async fn connect_in(base: &str, cwd: &std::path::Path) -> Result<Self, BridgeError> {
        let cwd = cwd
            .canonicalize()
            .map_err(|e| BridgeError::Transport(format!("bad cwd {}: {e}", cwd.display())))?;
        let cwd = cwd
            .to_str()
            .ok_or_else(|| BridgeError::Transport("cwd is not valid utf-8".into()))?
            .to_owned();
        let conn = Connection::open(base).await?;
        let init = conn
            .request(
                "initialize",
                serde_json::json!({ "protocolVersion": 1, "clientCapabilities": {} }),
            )
            .await?;
        let load_session = init
            .get("agentCapabilities")
            .and_then(|c| c.get("loadSession"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !load_session {
            return Err(BridgeError::Transport(
                "goose serve lacks loadSession capability; crash-resume unsupported".into(),
            ));
        }
        Ok(Self {
            conn,
            inboxes: Mutex::new(HashMap::new()),
            cwd,
            lazyboy_mcp: None,
        })
    }

    /// Point every new/loaded session at the lazyboy MCP server at `url`,
    /// presenting `token` as the bearer. This is what gives the agent its
    /// lazyboy tools; without it sessions open with `mcpServers: []` and
    /// the agent can only use goose's own tools.
    pub fn with_lazyboy_mcp(mut self, url: String, token: Option<String>) -> Self {
        self.lazyboy_mcp = Some(LazyboyMcp { url, token });
        self
    }

    /// The `mcpServers` array for a session scoped to `space_id`. Empty
    /// unless a lazyboy MCP endpoint was configured; when set, one HTTP
    /// entry carrying the space-binding header (and the bearer, if any).
    fn mcp_servers(&self, space_id: &str) -> serde_json::Value {
        let Some(mcp) = &self.lazyboy_mcp else {
            return serde_json::json!([]);
        };
        let mut headers = vec![serde_json::json!({
            "name": "X-Lazyboy-Space",
            "value": space_id,
        })];
        if let Some(token) = &mcp.token {
            headers.push(serde_json::json!({
                "name": "Authorization",
                "value": format!("Bearer {token}"),
            }));
        }
        serde_json::json!([{
            "type": "http",
            "name": "lazyboy",
            "url": mcp.url,
            "headers": headers,
        }])
    }

    fn register(&self, session: &str) {
        let rx = self.conn.subscribe(session);
        self.inboxes.lock().unwrap().insert(session.to_owned(), rx);
    }
}

fn session_from(result: &serde_json::Value) -> Result<String, BridgeError> {
    result
        .get("sessionId")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .ok_or_else(|| BridgeError::Transport("session result missing sessionId".into()))
}

#[async_trait]
impl GooseClient for GooseServeClient {
    async fn new_session(&self, space_id: &str) -> Result<SessionId, BridgeError> {
        let result = self
            .conn
            .request(
                "session/new",
                serde_json::json!({
                    "cwd": self.cwd,
                    "mcpServers": self.mcp_servers(space_id),
                }),
            )
            .await?;
        let id = session_from(&result)?;
        self.register(&id);
        Ok(SessionId(id))
    }

    async fn load_session(&self, session: &SessionId, space_id: &str) -> Result<(), BridgeError> {
        self.conn
            .request(
                "session/load",
                serde_json::json!({
                    "sessionId": session.0,
                    "cwd": self.cwd,
                    "mcpServers": self.mcp_servers(space_id),
                }),
            )
            .await?;
        self.register(&session.0);
        Ok(())
    }

    async fn prompt(&self, session: &SessionId, text: &str) -> Result<(), BridgeError> {
        // Fire-and-return: the driver awaits this, then pulls updates, so
        // it must not block until the turn ends. The `session/prompt`
        // response (a `stopReason`) is the turn boundary; the demux routes
        // it onto this session's queue as `TurnEnded` once it arrives,
        // after every streamed `session/update` for the turn (the WS is
        // ordered). A turn suspended on an approval never produces that
        // response until the decision is sent, which is exactly the
        // durable pause we want.
        self.conn
            .prompt(
                &session.0,
                serde_json::json!({
                    "sessionId": session.0,
                    "prompt": [ { "type": "text", "text": text } ],
                }),
            )
            .await
    }

    async fn next_update(&self, session: &SessionId) -> Result<Option<Update>, BridgeError> {
        // Take the receiver out under the lock, await off-lock, then put
        // it back. The trait is a blocking pull — `None` must mean the
        // stream truly closed (sender dropped: connection gone or session
        // ended), not "no frame yet"; the driver reads `None` as a drained
        // turn, so a non-blocking peek here would end live runs early.
        let mut rx = {
            let mut inboxes = self.inboxes.lock().unwrap();
            inboxes
                .remove(&session.0)
                .ok_or_else(|| BridgeError::UnknownSession(session.0.clone()))?
        };
        let update = rx.recv().await;
        self.inboxes.lock().unwrap().insert(session.0.clone(), rx);
        Ok(update)
    }

    async fn answer_permission(
        &self,
        _session: &SessionId,
        request_id: &str,
        decision: Decision,
    ) -> Result<(), BridgeError> {
        let id: u64 = request_id
            .parse()
            .map_err(|_| BridgeError::Transport(format!("bad request id: {request_id}")))?;
        let outcome = match decision {
            Decision::Allow => "selected",
            Decision::Deny => "cancelled",
        };
        // Answer the agent->client request goose is blocked on, echoing
        // its id back as the JSON-RPC response id.
        self.conn
            .respond(id, serde_json::json!({ "outcome": { "outcome": outcome } }))
            .await?;
        Ok(())
    }
}
