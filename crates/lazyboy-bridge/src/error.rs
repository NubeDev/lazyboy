/// Errors from the Goose seam or from importing its events.
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error(transparent)]
    Store(#[from] lazyboy_store::StoreError),

    /// The transport (HTTP/WebSocket) or ACP framing failed. The live
    /// host client maps connection and protocol faults here; FakeGoose
    /// uses it to simulate a dropped goosed.
    #[error("goose transport: {0}")]
    Transport(String),

    #[error("goose session {0} not found")]
    UnknownSession(String),
}
