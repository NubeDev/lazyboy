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

    /// A retry was asked for a run that has no recorded prompt event —
    /// only reachable if the run predates prompt recording or state was
    /// hand-edited.
    #[error("run {0} has no recorded prompt to retry")]
    NoPrompt(String),
}
