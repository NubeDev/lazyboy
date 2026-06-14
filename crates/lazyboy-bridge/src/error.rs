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

    /// Reading or writing the goose provider configuration (the
    /// lazyboy-owned config and secrets files the supervisor launches
    /// goose with) failed, or a settings write named an unknown provider.
    #[error("goose config: {0}")]
    Config(String),
}
