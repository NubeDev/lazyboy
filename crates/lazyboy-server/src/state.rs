use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use lazyboy_adapters_host::{GooseConfigStore, GooseServeClient, GooseSupervisor};
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
    /// The provider selection/secrets store and the goose process it
    /// launches, so the settings endpoints can read and apply a provider.
    goose_config: GooseConfigStore,
    goose: GooseSupervisor,
}

impl AppState {
    /// Build state for `goose_url`. The goose config store and supervisor
    /// are constructed here (matching `TauriRpc::new`) so every caller —
    /// `serve` and the route tests — gets the settings surface without
    /// threading the pieces through. `serve` separately drives
    /// `goose().restart()` at boot; tests never start the child, so the
    /// supervisor is inert unless a settings write asks it to launch. A
    /// config-dir resolution miss is unrecoverable (the shell cannot
    /// manage goose without it).
    pub fn new(store: Store, goose_url: String, token: Option<String>) -> Self {
        let goose_config =
            GooseConfigStore::discover().expect("resolve lazyboy config dir for goose settings");
        let binary = std::env::var("GOOSE_BIN").unwrap_or_else(|_| "bin/goose".to_owned());
        let goose = GooseSupervisor::new(binary, goose_serve_addr(&goose_url), goose_config.clone());
        Self {
            inner: Arc::new(Inner {
                store,
                goose_url,
                token,
                goose_config,
                goose,
            }),
        }
    }

    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    pub fn token(&self) -> Option<&str> {
        self.inner.token.as_deref()
    }

    pub fn goose_config(&self) -> &GooseConfigStore {
        &self.inner.goose_config
    }

    pub fn goose(&self) -> &GooseSupervisor {
        &self.inner.goose
    }

    /// The configured `goose serve` base, surfaced so the health probe
    /// can attempt a connection and report it to the UI.
    pub fn goose_url(&self) -> &str {
        &self.inner.goose_url
    }

    /// Build an engine for a mutating request: connect to goose afresh
    /// (the host transport is per-connection) and resolve the agent
    /// principal from the store. Reconcile is the caller's job after.
    pub async fn engine(&self) -> Result<Engine<GooseServeClient>, ApiError> {
        let agent = self.identity("agent").await?;
        let client = GooseServeClient::connect(&self.inner.goose_url)
            .await?
            .with_lazyboy_mcp(self.mcp_url(), self.inner.token.clone());
        Ok(Engine::new(self.inner.store.clone(), client, agent))
    }

    /// The lazyboy `/mcp` URL goose is told to connect back to so the
    /// agent gets its lazyboy tools. Derived from the server's own listen
    /// address (`LAZYBOY_ADDR`, the same env `main` binds), mirroring how
    /// the goose binary path is read from env here. goose runs on the
    /// same host, so loopback is reachable.
    fn mcp_url(&self) -> String {
        let addr = std::env::var("LAZYBOY_ADDR").unwrap_or_else(|_| "127.0.0.1:7878".to_owned());
        format!("http://{addr}/mcp")
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

/// Parse the `host:port` the supervised `goose serve` should bind from
/// the configured goose url, defaulting the port to goose's 3284 and
/// falling back to `127.0.0.1:3284` when the url cannot be resolved (the
/// constructor cannot surface an error and that is goose's own default).
fn goose_serve_addr(goose_url: &str) -> SocketAddr {
    let hostport = goose_url
        .rsplit("://")
        .next()
        .unwrap_or(goose_url)
        .trim_end_matches('/');
    let with_port = if hostport.contains(':') {
        hostport.to_owned()
    } else {
        format!("{hostport}:3284")
    };
    with_port
        .to_socket_addrs()
        .ok()
        .and_then(|mut a| a.next())
        .unwrap_or_else(|| ([127, 0, 0, 1], 3284).into())
}
