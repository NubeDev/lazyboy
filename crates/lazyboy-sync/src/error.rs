/// Errors crossing the sync boundary. Serde failures are the common
/// case (a malformed inbound event); the store and zenoh variants wrap
/// their sources so a caller can distinguish a local-write failure from
/// a transport failure.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Serde(#[from] serde_json::Error),

    #[error(transparent)]
    Store(#[from] lazyboy_store::StoreError),

    #[cfg(feature = "zenoh")]
    #[error("zenoh: {0}")]
    Zenoh(String),
}
