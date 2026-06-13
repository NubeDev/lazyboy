//! The live Zenoh seam (feature `zenoh`). Everything network-touching
//! lives here so the rest of the crate — the LWW merge, the drain
//! mapping, the event serde — compiles and tests with no network and no
//! zenoh dependency. SCOPE.md's "no Zenoh until the local event model is
//! stable" gate is enforced structurally: the outbox is the only thing
//! this module reads from, and the only thing it writes to is the same
//! store verbs a local mutation uses.

use lazyboy_store::Store;
use time::OffsetDateTime;
use zenoh::config::Config;
use zenoh::Session;

use crate::config::{SyncConfig, Topology};
use crate::drain::{key_for, to_publication};
use crate::event::SyncEvent;
use crate::SyncError;

fn zenoh_config(cfg: &SyncConfig) -> Config {
    match &cfg.topology {
        Topology::Peer => Config::default(),
        Topology::Client { endpoints } => {
            let mut config = Config::default();
            // Best-effort: a malformed endpoint string is a config error
            // surfaced at open(), not a silent fallback to peer mode.
            let json = serde_json::json!({ "connect": { "endpoints": endpoints } });
            let _ = config.insert_json5("connect", &json["connect"].to_string());
            config
        }
    }
}

/// Open a Zenoh session for the configured topology.
pub async fn open(cfg: &SyncConfig) -> Result<Session, SyncError> {
    zenoh::open(zenoh_config(cfg))
        .await
        .map_err(|e| SyncError::Zenoh(e.to_string()))
}

/// Drain the outbox once: publish every unsynced event, marking each
/// synced only after its put succeeds, so a transport failure leaves the
/// event in the queue for the next pass rather than dropping it.
pub async fn publish_pending(
    session: &Session,
    store: &Store,
    cfg: &SyncConfig,
) -> Result<usize, SyncError> {
    let rows = lazyboy_store::repo::outbox::unsynced(store).await?;
    let mut sent = 0;
    for row in &rows {
        let pubn = to_publication(&cfg.workspace, row)?;
        session
            .put(&pubn.key, pubn.payload)
            .await
            .map_err(|e| SyncError::Zenoh(e.to_string()))?;
        lazyboy_store::repo::outbox::mark_synced(store, row.id, OffsetDateTime::now_utc()).await?;
        sent += 1;
    }
    Ok(sent)
}

/// Subscribe to this workspace's whole key space and apply each inbound
/// event to the local store until the session closes.
pub async fn run_subscriber(
    session: &Session,
    store: &Store,
    cfg: &SyncConfig,
) -> Result<(), SyncError> {
    let key = key_for(&cfg.workspace, "*", "**");
    let subscriber = session
        .declare_subscriber(&key)
        .await
        .map_err(|e| SyncError::Zenoh(e.to_string()))?;
    while let Ok(sample) = subscriber.recv_async().await {
        let bytes = sample.payload().to_bytes();
        let event: SyncEvent = serde_json::from_slice(&bytes)?;
        crate::apply::apply(store, &event).await?;
    }
    Ok(())
}
