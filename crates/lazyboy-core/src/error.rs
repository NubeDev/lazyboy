/// Errors crossing the core orchestration boundary.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Store(#[from] lazyboy_store::StoreError),

    #[error(transparent)]
    Bridge(#[from] lazyboy_bridge::BridgeError),

    /// A run reached an approval point but the captured approval row
    /// names a tool whose request the bridge no longer holds the ACP
    /// id for — only reachable if state was hand-edited.
    #[error("run {0} has no pending permission to resume")]
    NoPendingPermission(String),
}
