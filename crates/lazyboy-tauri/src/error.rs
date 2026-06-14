use lazyboy_bridge::BridgeError;
use lazyboy_core::CoreError;
use lazyboy_store::StoreError;

/// The fault surface the desktop shell hands back to the webview. The
/// HTTP shell renders the same store/core/bridge faults as status codes;
/// here there is no status line, so the variants carry a stable message
/// the Tauri command layer serialises to the UI. A store lookup miss is
/// kept distinct so the UI can branch on it exactly as it branches on the
/// server's 404.
#[derive(Debug)]
pub enum RpcError {
    NotFound(String),
    /// A malformed argument the command could not act on (an RFC3339
    /// timestamp that won't parse, invalid integration bindings). The
    /// HTTP shell renders the same fault as a 400.
    BadRequest(String),
    Store(StoreError),
    Core(CoreError),
    Bridge(BridgeError),
}

impl From<StoreError> for RpcError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::NotFound(what) => RpcError::NotFound(what),
            other => RpcError::Store(other),
        }
    }
}

impl From<CoreError> for RpcError {
    fn from(e: CoreError) -> Self {
        RpcError::Core(e)
    }
}

impl From<BridgeError> for RpcError {
    fn from(e: BridgeError) -> Self {
        RpcError::Bridge(e)
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::NotFound(what) => write!(f, "not found: {what}"),
            RpcError::BadRequest(why) => write!(f, "bad request: {why}"),
            RpcError::Store(e) => write!(f, "{e}"),
            RpcError::Core(e) => write!(f, "{e}"),
            RpcError::Bridge(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RpcError {}
