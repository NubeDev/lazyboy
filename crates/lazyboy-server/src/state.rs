use std::sync::Arc;

use lazyboy_adapters_host::GooseServeClient;
use lazyboy_core::Engine;
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::Identity;
use lazyboy_types::Id;

use crate::error::ApiError;

/// Shared, cheap-to-clone server state. The store is the source of
/// truth for every read; the goose url and identity ids are only needed
/// to build a fresh `Engine` for the two mutating endpoints (startRun,
/// decide), exactly as the CLI builds one per command.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    store: Store,
    goose_url: String,
    /// Single-tenant bearer (SCOPE.md R4). `None` disables auth for dev.
    token: Option<String>,
}

impl AppState {
    pub fn new(store: Store, goose_url: String, token: Option<String>) -> Self {
        Self {
            inner: Arc::new(Inner {
                store,
                goose_url,
                token,
            }),
        }
    }

    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    pub fn token(&self) -> Option<&str> {
        self.inner.token.as_deref()
    }

    /// Build an engine for a mutating request: connect to goose afresh
    /// (the host transport is per-connection) and resolve the agent
    /// principal from the store. Reconcile is the caller's job after.
    pub async fn engine(&self) -> Result<Engine<GooseServeClient>, ApiError> {
        let agent = self.identity("agent").await?;
        let client = GooseServeClient::connect(&self.inner.goose_url).await?;
        Ok(Engine::new(self.inner.store.clone(), client, agent))
    }

    /// The human principal that authors decisions, looked up by kind so
    /// the server need not carry the CLI's config sidecar.
    pub async fn human(&self) -> Result<Id<Identity>, ApiError> {
        self.identity("human").await
    }

    /// The principal that authors ingress messages. Integration-sourced
    /// timeline rows are not human-authored; MVP attributes them to the
    /// node's `agent` principal rather than minting a per-integration
    /// identity (deferred with the rest of the membership model, R4).
    pub async fn ingress_author(&self) -> Result<Id<Identity>, ApiError> {
        self.identity("agent").await
    }

    async fn identity(&self, kind: &str) -> Result<Id<Identity>, ApiError> {
        repo::identity::find_by_kind(&self.inner.store, kind)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("identity kind '{kind}' (run init first)")))
    }
}
