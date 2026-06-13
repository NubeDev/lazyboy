use async_trait::async_trait;

use super::{Decision, SessionId, Update};
use crate::BridgeError;

use std::sync::Arc;

/// The whole Goose surface Lazyboy depends on. Implemented by the host
/// HTTP+WebSocket transport in production and by `FakeGoose` in tests.
/// Keeping it this narrow is what makes "never fork Goose" enforceable:
/// the product never reaches past these five calls.
#[async_trait]
pub trait GooseClient: Send + Sync {
    /// Open a fresh session and return its id (`session/new`).
    async fn new_session(&self) -> Result<SessionId, BridgeError>;

    /// Re-attach to an existing session after a crash (`session/load`,
    /// gated by the agent's `loadSession` capability). The driver then
    /// re-reads `next_update` from goose's persisted history.
    async fn load_session(&self, session: &SessionId) -> Result<(), BridgeError>;

    /// Send a user prompt into a session (`session/prompt`).
    async fn prompt(&self, session: &SessionId, text: &str) -> Result<(), BridgeError>;

    /// Pull the next update for a session, or `None` when the stream is
    /// exhausted. Maps `session/update` notifications and
    /// `session/request_permission` requests off the WebSocket.
    async fn next_update(&self, session: &SessionId) -> Result<Option<Update>, BridgeError>;

    /// Answer an outstanding permission request, releasing or killing
    /// the gated tool (the reply to `session/request_permission`).
    async fn answer_permission(
        &self,
        session: &SessionId,
        request_id: &str,
        decision: Decision,
    ) -> Result<(), BridgeError>;
}

/// Let a shared client be driven through an `Arc`, so the engine and a
/// test (or the live transport's reconnect task) can hold the same
/// connection without it being `Clone`.
#[async_trait]
impl<G: GooseClient> GooseClient for Arc<G> {
    async fn new_session(&self) -> Result<SessionId, BridgeError> {
        (**self).new_session().await
    }
    async fn load_session(&self, session: &SessionId) -> Result<(), BridgeError> {
        (**self).load_session(session).await
    }
    async fn prompt(&self, session: &SessionId, text: &str) -> Result<(), BridgeError> {
        (**self).prompt(session, text).await
    }
    async fn next_update(&self, session: &SessionId) -> Result<Option<Update>, BridgeError> {
        (**self).next_update(session).await
    }
    async fn answer_permission(
        &self,
        session: &SessionId,
        request_id: &str,
        decision: Decision,
    ) -> Result<(), BridgeError> {
        (**self)
            .answer_permission(session, request_id, decision)
            .await
    }
}
