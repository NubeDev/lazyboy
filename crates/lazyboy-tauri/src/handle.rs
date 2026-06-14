use std::sync::Arc;

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use lazyboy_adapters_host::{GooseConfigStore, GooseServeClient, GooseSupervisor};
use lazyboy_core::Engine;
use lazyboy_ingress::{self, Bindings};
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{
    Approval, ApprovalStatus, Group, Identity, Integration, Reminder, ReminderStatus, Space,
    Workflow, WorkflowStatus, Workspace,
};
use lazyboy_types::Id;
use lazyboy_wire::{
    ApprovalDto, CalendarEventDto, CreateIntegrationBody, CreateReminderBody, CreateSpaceBody,
    CreateTaskBody, CreateWorkflowBody,
    CreatedIdDto, DecisionDto, IngestResultDto, IntegrationDto, MessageDto, RecordDecisionBody,
    ReminderDto, RunDto, RunOutcomeDto, SpaceDto, TaskDto, UpsertCalendarBody, WorkflowDto,
};

use crate::error::RpcError;

/// Parse an RFC3339 instant the webview sent, mapping a malformed value
/// to the same `BadRequest` fault the HTTP shell returns as a 400.
fn parse_rfc3339(label: &str, value: &str) -> Result<OffsetDateTime, RpcError> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|e| RpcError::BadRequest(format!("{label} not rfc3339: {e}")))
}

/// Parse the `host:port` the supervised `goose serve` should bind from
/// the configured goose url, defaulting the port to goose's 3284. Falls
/// back to `127.0.0.1:3284` if the url cannot be resolved, since the
/// constructor cannot surface an error and that is goose's own default.
fn goose_serve_addr(goose_url: &str) -> std::net::SocketAddr {
    use std::net::ToSocketAddrs;
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

/// Decode an integration's `config_json` ingress bindings, mirroring the
/// HTTP handler: absent config is the empty binding set, malformed config
/// is a `BadRequest`.
fn parse_bindings(config_json: Option<&str>) -> Result<Bindings, RpcError> {
    match config_json {
        None => Ok(Bindings::default()),
        Some(raw) => serde_json::from_str(raw)
            .map_err(|e| RpcError::BadRequest(format!("integration config_json invalid: {e}"))),
    }
}

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
    goose_config: GooseConfigStore,
    goose: GooseSupervisor,
}

impl TauriRpc {
    pub fn new(store: Store, goose_url: String) -> Self {
        // The desktop shell owns goose exactly as the server does: a
        // config store for the provider selection and a supervisor that
        // launches `goose serve` on the configured url's host:port. The
        // binary defaults to the vendored `bin/goose`, overridable with
        // GOOSE_BIN. A config-dir resolution miss is unrecoverable here
        // (the shell cannot manage goose without it), so it is a hard
        // failure rather than a degraded state.
        let goose_config =
            GooseConfigStore::discover().expect("resolve lazyboy config dir for goose settings");
        let addr = goose_serve_addr(&goose_url);
        let binary = std::env::var("GOOSE_BIN").unwrap_or_else(|_| "bin/goose".to_owned());
        let goose = GooseSupervisor::new(binary, addr, goose_config.clone());
        Self {
            inner: Arc::new(Inner {
                store,
                goose_url,
                goose_config,
                goose,
            }),
        }
    }

    pub fn store(&self) -> &Store {
        &self.inner.store
    }

    /// Bring goose up under the saved provider; the desktop entrypoint
    /// calls this once at startup. Non-fatal: a missing provider is the
    /// first-run state, fixed from the settings UI.
    pub async fn start_goose(&self) {
        match self.inner.goose.restart().await {
            Ok(()) => {}
            Err(e) => tracing::warn!(%e, "goose not started; configure a provider in settings"),
        }
    }

    /// `list_spaces` -> `Space[]` (RpcClient.listSpaces).
    pub async fn list_spaces(&self) -> Result<Vec<SpaceDto>, RpcError> {
        let rows = repo::space::list(self.store()).await?;
        Ok(rows.into_iter().map(SpaceDto::from).collect())
    }

    /// `create_space` `{slug, title}` -> the created `Space`
    /// (RpcClient.createSpace). The workspace is resolved here (single
    /// trust boundary, SCOPE R5); a slug already in use is a `BadRequest`,
    /// mirroring the HTTP shell's 400.
    pub async fn create_space(&self, body: CreateSpaceBody) -> Result<SpaceDto, RpcError> {
        let slug = body.slug.trim();
        if slug.is_empty() {
            return Err(RpcError::BadRequest("slug must not be empty".to_owned()));
        }
        let workspace_id = repo::workspace::current(self.store()).await?;
        let id = repo::bootstrap::create_space(self.store(), workspace_id, slug, body.title.trim())
            .await
            .map_err(|e| {
                if e.is_unique_violation() {
                    RpcError::BadRequest(format!("slug '{slug}' already in use"))
                } else {
                    RpcError::from(e)
                }
            })?;
        let rows = repo::space::list(self.store()).await?;
        rows.into_iter()
            .find(|r| r.id == id)
            .map(SpaceDto::from)
            .ok_or_else(|| RpcError::NotFound("space vanished after create".to_owned()))
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

    /// `create_task` `{title}` -> the created `Task` (RpcClient.createTask).
    /// Deterministic quick-add: opens a task with no agent run, mirroring
    /// the HTTP shell's `POST /spaces/:id/tasks`.
    pub async fn create_task(
        &self,
        space_id: Id<Space>,
        body: CreateTaskBody,
    ) -> Result<TaskDto, RpcError> {
        let title = body.title.trim();
        if title.is_empty() {
            return Err(RpcError::BadRequest("task title is empty".to_owned()));
        }
        let id = repo::task::create(self.store(), space_id, title, None).await?;
        let rows = repo::task::list(self.store(), space_id).await?;
        let row = rows
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| RpcError::BadRequest("task vanished after create".to_owned()))?;
        Ok(row.into())
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
        let started = engine.start_chat(space_id, prompt).await?;
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

    /// `health` -> goose reachability. Probes with the same
    /// connect/initialize handshake a mutating call uses, so the desktop
    /// status reflects whether work could run now. A failed probe is a
    /// populated `goose_reachable: false`, not an error: the in-process
    /// node is healthy, goose is the dependency that is down.
    pub async fn health(&self) -> Result<lazyboy_wire::HealthDto, RpcError> {
        let (goose_reachable, goose_detail) =
            match GooseServeClient::connect(&self.inner.goose_url).await {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            };
        Ok(lazyboy_wire::HealthDto {
            goose_url: self.inner.goose_url.clone(),
            goose_reachable,
            goose_detail,
        })
    }

    /// `list_goose_providers` -> selectable providers with `key_set`
    /// flags (never the key itself, SCOPE.md R5).
    pub async fn list_goose_providers(
        &self,
    ) -> Result<Vec<lazyboy_wire::GooseProviderDto>, RpcError> {
        let store = &self.inner.goose_config;
        let mut out = Vec::with_capacity(lazyboy_adapters_host::PROVIDERS.len());
        for p in lazyboy_adapters_host::PROVIDERS {
            out.push(lazyboy_wire::GooseProviderDto {
                id: p.id.to_owned(),
                display_name: p.display_name.to_owned(),
                requires_key: p.requires_key,
                key_set: store.has_key(p.key_env)?,
                models: p.models.iter().map(|m| (*m).to_owned()).collect(),
            });
        }
        Ok(out)
    }

    /// `get_goose_config` -> the current selection and live process state.
    pub async fn get_goose_config(&self) -> Result<lazyboy_wire::GooseConfigDto, RpcError> {
        let selection = self.inner.goose_config.selection()?;
        Ok(lazyboy_wire::GooseConfigDto {
            provider: selection.provider,
            model: selection.model,
            running: self.inner.goose.running().await,
        })
    }

    /// `set_goose_config` -> persist the selection (and key when given),
    /// relaunch goose, and report the applied config. A relaunch failure
    /// surfaces with the selection already saved, exactly as the HTTP
    /// shell.
    pub async fn set_goose_config(
        &self,
        provider: &str,
        model: Option<&str>,
        api_key: Option<&str>,
    ) -> Result<lazyboy_wire::GooseConfigDto, RpcError> {
        let selection = self.inner.goose_config.save(provider, model, api_key)?;
        self.inner.goose.restart().await?;
        Ok(lazyboy_wire::GooseConfigDto {
            provider: selection.provider,
            model: selection.model,
            running: self.inner.goose.running().await,
        })
    }

    /// `list_decisions` -> `Decision[]`.
    pub async fn list_decisions(&self, space_id: Id<Space>) -> Result<Vec<DecisionDto>, RpcError> {
        let rows = repo::decision::list(self.store(), space_id).await?;
        Ok(rows.into_iter().map(DecisionDto::from).collect())
    }

    /// `record_decision` -> the recorded `Decision`.
    pub async fn record_decision(
        &self,
        space_id: Id<Space>,
        body: RecordDecisionBody,
    ) -> Result<DecisionDto, RpcError> {
        let id = repo::decision::record(
            self.store(),
            repo::decision::NewDecision {
                space_id,
                message_id: body.message_id,
                summary: &body.summary,
                decided_by_identity_id: body.decided_by_identity_id,
            },
        )
        .await?;
        let rows = repo::decision::list(self.store(), space_id).await?;
        rows.into_iter()
            .find(|r| r.id == id)
            .map(DecisionDto::from)
            .ok_or_else(|| RpcError::NotFound("decision vanished after record".to_owned()))
    }

    /// `list_reminders` -> `Reminder[]`.
    pub async fn list_reminders(&self, space_id: Id<Space>) -> Result<Vec<ReminderDto>, RpcError> {
        let rows = repo::reminder::list(self.store(), space_id).await?;
        Ok(rows.into_iter().map(ReminderDto::from).collect())
    }

    /// `create_reminder` -> the created `Reminder`. `due_at` is RFC3339.
    pub async fn create_reminder(
        &self,
        space_id: Id<Space>,
        body: CreateReminderBody,
    ) -> Result<ReminderDto, RpcError> {
        let due_at = parse_rfc3339("due_at", &body.due_at)?;
        let id = repo::reminder::create(
            self.store(),
            repo::reminder::NewReminder {
                space_id,
                task_id: body.task_id,
                due_at,
                body: &body.body,
            },
        )
        .await?;
        let rows = repo::reminder::list(self.store(), space_id).await?;
        rows.into_iter()
            .find(|r| r.id == id)
            .map(ReminderDto::from)
            .ok_or_else(|| RpcError::NotFound("reminder vanished after create".to_owned()))
    }

    /// `dismiss_reminder` -> the dismissed `Reminder`. A dismiss of an
    /// unknown or already-settled reminder is a `NotFound`, mirroring the
    /// HTTP shell's 404, so a racing second click is reported.
    pub async fn dismiss_reminder(
        &self,
        reminder_id: Id<Reminder>,
    ) -> Result<ReminderDto, RpcError> {
        let changed =
            repo::reminder::set_status(self.store(), reminder_id, ReminderStatus::Dismissed)
                .await?;
        if !changed {
            return Err(RpcError::NotFound("reminder not found".to_owned()));
        }
        let row = repo::reminder::get(self.store(), reminder_id)
            .await?
            .ok_or_else(|| RpcError::NotFound("reminder vanished after dismiss".to_owned()))?;
        Ok(row.into())
    }

    /// `list_calendar` -> `CalendarEvent[]`.
    pub async fn list_calendar(
        &self,
        space_id: Id<Space>,
    ) -> Result<Vec<CalendarEventDto>, RpcError> {
        let rows =
            repo::calendar::list(self.store(), space_id, repo::calendar::Window::default()).await?;
        Ok(rows.into_iter().map(CalendarEventDto::from).collect())
    }

    /// `upsert_calendar` -> the upserted event. A synced event re-sent
    /// with the same `(source, external_ref)` refreshes the row.
    pub async fn upsert_calendar(
        &self,
        space_id: Id<Space>,
        body: UpsertCalendarBody,
    ) -> Result<CalendarEventDto, RpcError> {
        let starts_at = parse_rfc3339("starts_at", &body.starts_at)?;
        let ends_at = body
            .ends_at
            .as_deref()
            .map(|s| parse_rfc3339("ends_at", s))
            .transpose()?;
        let id = repo::calendar::upsert(
            self.store(),
            repo::calendar::NewCalendarEvent {
                space_id,
                source: &body.source,
                external_ref: body.external_ref.as_deref(),
                title: &body.title,
                starts_at,
                ends_at,
                meta_json: body.meta_json.as_deref(),
            },
        )
        .await?;
        let rows =
            repo::calendar::list(self.store(), space_id, repo::calendar::Window::default()).await?;
        rows.into_iter()
            .find(|r| r.id == id)
            .map(CalendarEventDto::from)
            .ok_or_else(|| RpcError::NotFound("calendar event vanished after upsert".to_owned()))
    }

    /// `list_integrations` -> `Integration[]`.
    pub async fn list_integrations(
        &self,
        workspace_id: Id<Workspace>,
    ) -> Result<Vec<IntegrationDto>, RpcError> {
        let rows = repo::integration::list(self.store(), workspace_id).await?;
        Ok(rows.into_iter().map(IntegrationDto::from).collect())
    }

    /// `create_integration` -> the created `Integration`. The body carries
    /// only a `secret_ref` (a host secrets-store pointer), never a raw
    /// secret (SCOPE.md R5).
    pub async fn create_integration(
        &self,
        body: CreateIntegrationBody,
    ) -> Result<IntegrationDto, RpcError> {
        let config_json = body.config_json.as_ref().map(ToString::to_string);
        let id = repo::integration::create(
            self.store(),
            repo::integration::NewIntegration {
                workspace_id: body.workspace_id,
                provider: body.provider,
                account_ref: body.account_ref.as_deref(),
                secret_ref: body.secret_ref.as_deref(),
                config_json: config_json.as_deref(),
            },
        )
        .await?;
        let row = repo::integration::get(self.store(), id)
            .await?
            .ok_or_else(|| RpcError::NotFound("integration vanished after create".to_owned()))?;
        Ok(row.into())
    }

    /// `ingress` -> `{message_id, deduped}`. Normalizes a raw provider
    /// payload, resolves the bound space (explicit `space_id`, else the
    /// integration's `config_json` bindings), then dedups on
    /// `(integration_id, external_id)`, mirroring the HTTP handler.
    pub async fn ingress(
        &self,
        integration_id: Id<Integration>,
        payload: serde_json::Value,
        space_id: Option<Id<Space>>,
    ) -> Result<IngestResultDto, RpcError> {
        let integration = repo::integration::get(self.store(), integration_id)
            .await?
            .ok_or_else(|| RpcError::NotFound("integration".to_owned()))?;

        let event = lazyboy_ingress::normalize(integration.provider, &payload)
            .map_err(|e| RpcError::BadRequest(e.to_string()))?;

        let space_id = match space_id {
            Some(space_id) => space_id,
            None => {
                let bindings = parse_bindings(integration.config_json.as_deref())?;
                lazyboy_ingress::resolve_space(&bindings, &payload).ok_or_else(|| {
                    RpcError::BadRequest(
                        "no space_id given and no config_json binding matched the payload"
                            .to_owned(),
                    )
                })?
            }
        };

        let author = self.identity("agent").await?;
        let payload_json = payload.to_string();
        let outcome = repo::ingress::ingest(
            self.store(),
            repo::ingress::NewIngress {
                integration_id,
                space_id,
                author,
                external_id: &event.external_id,
                kind: &event.kind,
                payload_json: &payload_json,
                body: &event.body,
            },
        )
        .await?;
        Ok(IngestResultDto {
            message_id: outcome.message_id,
            deduped: outcome.deduped,
        })
    }

    /// `set_feed_visibility` -> the created visibility row id.
    pub async fn set_feed_visibility(
        &self,
        feed_integration_id: Id<Integration>,
        space_id: Id<Space>,
        principal_kind: &str,
        principal_id: &str,
        mode: &str,
    ) -> Result<CreatedIdDto, RpcError> {
        let id = repo::membership::set_feed_visibility(
            self.store(),
            feed_integration_id,
            space_id,
            principal_kind,
            principal_id,
            mode,
        )
        .await?;
        Ok(CreatedIdDto { id })
    }

    /// `list_workflows` -> `Workflow[]`.
    pub async fn list_workflows(
        &self,
        workspace_id: Id<Workspace>,
    ) -> Result<Vec<WorkflowDto>, RpcError> {
        let rows = repo::workflow::list(self.store(), workspace_id).await?;
        Ok(rows.into_iter().map(WorkflowDto::from).collect())
    }

    /// `create_workflow` -> the created `Workflow` (disabled until armed).
    pub async fn create_workflow(
        &self,
        body: CreateWorkflowBody,
    ) -> Result<WorkflowDto, RpcError> {
        let id = repo::workflow::create(
            self.store(),
            repo::workflow::NewWorkflow {
                workspace_id: body.workspace_id,
                name: &body.name,
                trigger_kind: body.trigger_kind,
                trigger_config_json: body.trigger_config_json.as_deref(),
                approval_policy: body.approval_policy,
                steps_json: &body.steps_json,
            },
        )
        .await?;
        let row = repo::workflow::get(self.store(), id).await?;
        Ok(row.into())
    }

    /// `enable_workflow` -> the armed `Workflow`.
    pub async fn enable_workflow(
        &self,
        workflow_id: Id<Workflow>,
    ) -> Result<WorkflowDto, RpcError> {
        self.set_workflow_status(workflow_id, WorkflowStatus::Enabled)
            .await
    }

    /// `disable_workflow` -> the disarmed `Workflow`.
    pub async fn disable_workflow(
        &self,
        workflow_id: Id<Workflow>,
    ) -> Result<WorkflowDto, RpcError> {
        self.set_workflow_status(workflow_id, WorkflowStatus::Disabled)
            .await
    }

    async fn set_workflow_status(
        &self,
        workflow_id: Id<Workflow>,
        status: WorkflowStatus,
    ) -> Result<WorkflowDto, RpcError> {
        repo::workflow::set_status(self.store(), workflow_id, status).await?;
        let row = repo::workflow::get(self.store(), workflow_id).await?;
        Ok(row.into())
    }

    /// `fire_workflow` -> `RunOutcome`. Reconcile first so a fresh process
    /// clears any in-flight approval before opening new work, matching the
    /// HTTP shell.
    pub async fn fire_workflow(
        &self,
        workflow_id: Id<Workflow>,
        space_id: Id<Space>,
    ) -> Result<RunOutcomeDto, RpcError> {
        let engine = self.engine().await?;
        engine.reconcile().await?;
        let outcome = engine.run_workflow(workflow_id, space_id).await?;
        Ok(outcome.into())
    }

    /// `create_group` -> the created `Group`.
    pub async fn create_group(
        &self,
        workspace_id: Id<Workspace>,
        name: &str,
    ) -> Result<lazyboy_wire::GroupDto, RpcError> {
        let id = repo::membership::create_group(self.store(), workspace_id, name).await?;
        Ok(lazyboy_wire::GroupDto {
            id,
            workspace_id,
            name: name.to_owned(),
        })
    }

    /// `add_group_member`. Modeled membership, not enforced in the MVP
    /// trust gate under R4.
    pub async fn add_group_member(
        &self,
        group_id: Id<Group>,
        identity_id: Id<Identity>,
    ) -> Result<(), RpcError> {
        repo::membership::add_member(self.store(), group_id, identity_id).await?;
        Ok(())
    }

    /// `list_members` -> `Membership[]`, the read side of a grant.
    pub async fn list_members(
        &self,
        space_id: Id<Space>,
    ) -> Result<Vec<lazyboy_wire::MembershipDto>, RpcError> {
        let rows = repo::membership::list_memberships(self.store(), space_id).await?;
        Ok(rows.into_iter().map(lazyboy_wire::MembershipDto::from).collect())
    }

    /// `grant_membership` -> the created membership row id.
    pub async fn grant_membership(
        &self,
        space_id: Id<Space>,
        principal_kind: &str,
        principal_id: &str,
        role: &str,
    ) -> Result<CreatedIdDto, RpcError> {
        let id = repo::membership::grant_membership(
            self.store(),
            space_id,
            principal_kind,
            principal_id,
            role,
        )
        .await?;
        Ok(CreatedIdDto { id })
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
