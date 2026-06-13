use std::sync::Arc;

use lazyboy_adapters_host::GooseServeClient;
use lazyboy_core::Engine;
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{Approval, ApprovalStatus, Identity, Space};
use lazyboy_types::Id;
use lazyboy_wire::{ApprovalDto, MessageDto, RunDto, RunOutcomeDto, SpaceDto, TaskDto};

use crate::error::RpcError;

/// The desktop shell's in-process backend, mirroring the HTTP shell's
/// `AppState`: the store is the source of truth for every read (SCOPE.md
/// R1), and the goose url plus the agent principal build a fresh `Engine`
/// per mutating call, exactly as the server and CLI do.
///
/// These methods are the bodies the `#[tauri::command]` wrappers call;
/// they are kept free of any tauri type so they build and test on default
/// features without the GUI stack (see `app.rs`). The returned DTOs are
/// the shared `lazyboy-wire` shapes, so the JSON the webview receives is
/// byte-identical to what the HTTP shell emits.
#[derive(Clone)]
pub struct TauriRpc {
    inner: Arc<Inner>,
}

struct Inner {
    store: Store,
    goose_url: String,
}

impl TauriRpc {
    pub fn new(store: Store, goose_url: String) -> Self {
        Self {
            inner: Arc::new(Inner { store, goose_url }),
        }
    }

    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    /// `list_spaces` -> `Space[]` (RpcClient.listSpaces).
    pub async fn list_spaces(&self) -> Result<Vec<SpaceDto>, RpcError> {
        let rows = repo::space::list(self.store()).await?;
        Ok(rows.into_iter().map(SpaceDto::from).collect())
    }

    /// `timeline` -> `Message[]` (RpcClient.timeline).
    pub async fn timeline(&self, space_id: Id<Space>) -> Result<Vec<MessageDto>, RpcError> {
        let rows = repo::message::list(self.store(), space_id).await?;
        Ok(rows.into_iter().map(MessageDto::from).collect())
    }

    /// `list_pending` -> `Approval[]` (RpcClient.listPending).
    pub async fn list_pending(&self, space_id: Id<Space>) -> Result<Vec<ApprovalDto>, RpcError> {
        let rows = repo::approval::list_pending(self.store(), space_id).await?;
        Ok(rows.into_iter().map(ApprovalDto::from).collect())
    }

    /// `list_tasks` -> `Task[]` (RpcClient.listTasks).
    pub async fn list_tasks(&self, space_id: Id<Space>) -> Result<Vec<TaskDto>, RpcError> {
        let rows = repo::task::list(self.store(), space_id).await?;
        Ok(rows.into_iter().map(TaskDto::from).collect())
    }

    /// `list_runs` -> `AgentRun[]` (RpcClient.listRuns).
    pub async fn list_runs(&self, space_id: Id<Space>) -> Result<Vec<RunDto>, RpcError> {
        let rows = repo::run::list(self.store(), space_id).await?;
        Ok(rows.into_iter().map(RunDto::from).collect())
    }

    /// `start_run` `{prompt}` -> `RunOutcome` (RpcClient.startRun).
    /// Reconcile first so a run started in a fresh process after a crash
    /// re-drives any in-flight approval before opening new work, matching
    /// the HTTP shell and the CLI's `run`.
    pub async fn start_run(
        &self,
        space_id: Id<Space>,
        prompt: &str,
    ) -> Result<RunOutcomeDto, RpcError> {
        let engine = self.engine().await?;
        engine.reconcile().await?;
        let started = engine.start_run(space_id, prompt, prompt).await?;
        Ok(started.outcome.into())
    }

    /// `decide` `{status}` -> `RunOutcome` (RpcClient.decide). Reconcile
    /// first so the decision lands even in a fresh process after a crash,
    /// matching the HTTP shell and the CLI's `decide`.
    pub async fn decide(
        &self,
        approval_id: Id<Approval>,
        status: ApprovalStatus,
    ) -> Result<RunOutcomeDto, RpcError> {
        let engine = self.engine().await?;
        engine.reconcile().await?;
        let human = self.human().await?;
        match engine.resolve_approval(approval_id, status, human).await? {
            Some(outcome) => Ok(outcome.into()),
            None => Ok(RunOutcomeDto::AlreadyResolved),
        }
    }

    /// Build an engine for a mutating call: the host goose transport is
    /// per-connection, so connect afresh and resolve the agent principal
    /// from the store, exactly as `AppState::engine` does.
    async fn engine(&self) -> Result<Engine<GooseServeClient>, RpcError> {
        let agent = self.identity("agent").await?;
        let client = GooseServeClient::connect(&self.inner.goose_url).await?;
        Ok(Engine::new(self.inner.store.clone(), client, agent))
    }

    async fn human(&self) -> Result<Id<Identity>, RpcError> {
        self.identity("human").await
    }

    async fn identity(&self, kind: &str) -> Result<Id<Identity>, RpcError> {
        repo::identity::find_by_kind(self.store(), kind)
            .await?
            .ok_or_else(|| RpcError::NotFound(format!("identity kind '{kind}' (run init first)")))
    }
}
