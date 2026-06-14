//! The Tauri 2 wiring: `#[tauri::command]` wrappers over `TauriRpc`, the
//! per-space event-channel poller, and the `Builder`. Gated behind the
//! `app` feature so the GUI/webview stack and its system dependencies are
//! only pulled in for an actual desktop build; the command bodies in
//! `handle.rs` and the cursor logic in `subscribe.rs` build and test
//! without it.
//!
//! Command names and payloads match what the UI invokes in
//! `ui/lazyboy-ui/src/shell/tauri.ts`: `list_spaces`, `timeline`,
//! `list_pending`, `list_tasks`, `list_runs`, `start_run`, `decide`. The
//! webview subscribes by listening on the `space:{id}` event the poller
//! emits, the desktop mirror of the HTTP shell's SSE stream.

use std::time::Duration;

use tauri::{Emitter, Manager, State};

use lazyboy_types::domain::{
    Approval, ApprovalStatus, Group, Identity, Integration, Reminder, Space, Workflow, Workspace,
};
use lazyboy_types::Id;
use lazyboy_wire::{
    ApprovalDto, CalendarEventDto, CreateIntegrationBody, CreateReminderBody, CreateSpaceBody,
    CreateTaskBody, CreateWorkflowBody,
    CreatedIdDto, DecisionDto, GroupDto, IngestResultDto, IntegrationDto, MessageDto,
    RecordDecisionBody, ReminderDto, RunDto, RunOutcomeDto, SpaceDto, TaskDto, UpsertCalendarBody,
    WorkflowDto,
};

use crate::handle::TauriRpc;
use crate::subscribe::{new_messages_since, Cursor};

const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Tauri passes command errors to the webview by serialising the `Err`
/// to a string; `RpcError`'s `Display` carries the stable message.
type CmdResult<T> = Result<T, String>;

fn err(e: crate::error::RpcError) -> String {
    e.to_string()
}

#[tauri::command]
async fn list_spaces(rpc: State<'_, TauriRpc>) -> CmdResult<Vec<SpaceDto>> {
    rpc.list_spaces().await.map_err(err)
}

#[tauri::command]
async fn create_space(rpc: State<'_, TauriRpc>, body: CreateSpaceBody) -> CmdResult<SpaceDto> {
    rpc.create_space(body).await.map_err(err)
}

#[tauri::command]
async fn timeline(rpc: State<'_, TauriRpc>, space_id: Id<Space>) -> CmdResult<Vec<MessageDto>> {
    rpc.timeline(space_id).await.map_err(err)
}

#[tauri::command]
async fn list_pending(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
) -> CmdResult<Vec<ApprovalDto>> {
    rpc.list_pending(space_id).await.map_err(err)
}

#[tauri::command]
async fn list_tasks(rpc: State<'_, TauriRpc>, space_id: Id<Space>) -> CmdResult<Vec<TaskDto>> {
    rpc.list_tasks(space_id).await.map_err(err)
}

#[tauri::command]
async fn create_task(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
    body: CreateTaskBody,
) -> CmdResult<TaskDto> {
    rpc.create_task(space_id, body).await.map_err(err)
}

#[tauri::command]
async fn list_runs(rpc: State<'_, TauriRpc>, space_id: Id<Space>) -> CmdResult<Vec<RunDto>> {
    rpc.list_runs(space_id).await.map_err(err)
}

#[tauri::command]
async fn start_run(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
    prompt: String,
) -> CmdResult<RunOutcomeDto> {
    rpc.start_run(space_id, &prompt).await.map_err(err)
}

#[tauri::command]
async fn decide(
    rpc: State<'_, TauriRpc>,
    approval_id: Id<Approval>,
    status: ApprovalStatus,
) -> CmdResult<RunOutcomeDto> {
    rpc.decide(approval_id, status).await.map_err(err)
}

#[tauri::command]
async fn health(rpc: State<'_, TauriRpc>) -> CmdResult<lazyboy_wire::HealthDto> {
    rpc.health().await.map_err(err)
}

#[tauri::command]
async fn list_goose_providers(
    rpc: State<'_, TauriRpc>,
) -> CmdResult<Vec<lazyboy_wire::GooseProviderDto>> {
    rpc.list_goose_providers().await.map_err(err)
}

#[tauri::command]
async fn get_goose_config(rpc: State<'_, TauriRpc>) -> CmdResult<lazyboy_wire::GooseConfigDto> {
    rpc.get_goose_config().await.map_err(err)
}

#[tauri::command]
async fn set_goose_config(
    rpc: State<'_, TauriRpc>,
    provider: String,
    model: Option<String>,
    api_key: Option<String>,
) -> CmdResult<lazyboy_wire::GooseConfigDto> {
    rpc.set_goose_config(&provider, model.as_deref(), api_key.as_deref())
        .await
        .map_err(err)
}

#[tauri::command]
async fn list_decisions(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
) -> CmdResult<Vec<DecisionDto>> {
    rpc.list_decisions(space_id).await.map_err(err)
}

#[tauri::command]
async fn record_decision(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
    body: RecordDecisionBody,
) -> CmdResult<DecisionDto> {
    rpc.record_decision(space_id, body).await.map_err(err)
}

#[tauri::command]
async fn list_reminders(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
) -> CmdResult<Vec<ReminderDto>> {
    rpc.list_reminders(space_id).await.map_err(err)
}

#[tauri::command]
async fn create_reminder(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
    body: CreateReminderBody,
) -> CmdResult<ReminderDto> {
    rpc.create_reminder(space_id, body).await.map_err(err)
}

#[tauri::command]
async fn dismiss_reminder(
    rpc: State<'_, TauriRpc>,
    reminder_id: Id<Reminder>,
) -> CmdResult<ReminderDto> {
    rpc.dismiss_reminder(reminder_id).await.map_err(err)
}

#[tauri::command]
async fn list_calendar(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
) -> CmdResult<Vec<CalendarEventDto>> {
    rpc.list_calendar(space_id).await.map_err(err)
}

#[tauri::command]
async fn upsert_calendar(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
    body: UpsertCalendarBody,
) -> CmdResult<CalendarEventDto> {
    rpc.upsert_calendar(space_id, body).await.map_err(err)
}

#[tauri::command]
async fn list_integrations(
    rpc: State<'_, TauriRpc>,
    workspace_id: Id<Workspace>,
) -> CmdResult<Vec<IntegrationDto>> {
    rpc.list_integrations(workspace_id).await.map_err(err)
}

#[tauri::command]
async fn create_integration(
    rpc: State<'_, TauriRpc>,
    body: CreateIntegrationBody,
) -> CmdResult<IntegrationDto> {
    rpc.create_integration(body).await.map_err(err)
}

#[tauri::command]
async fn ingress(
    rpc: State<'_, TauriRpc>,
    integration_id: Id<Integration>,
    payload: serde_json::Value,
    space_id: Option<Id<Space>>,
) -> CmdResult<IngestResultDto> {
    rpc.ingress(integration_id, payload, space_id)
        .await
        .map_err(err)
}

#[tauri::command]
async fn set_feed_visibility(
    rpc: State<'_, TauriRpc>,
    integration_id: Id<Integration>,
    space_id: Id<Space>,
    principal_kind: String,
    principal_id: String,
    mode: String,
) -> CmdResult<CreatedIdDto> {
    rpc.set_feed_visibility(integration_id, space_id, &principal_kind, &principal_id, &mode)
        .await
        .map_err(err)
}

#[tauri::command]
async fn list_workflows(
    rpc: State<'_, TauriRpc>,
    workspace_id: Id<Workspace>,
) -> CmdResult<Vec<WorkflowDto>> {
    rpc.list_workflows(workspace_id).await.map_err(err)
}

#[tauri::command]
async fn create_workflow(
    rpc: State<'_, TauriRpc>,
    body: CreateWorkflowBody,
) -> CmdResult<WorkflowDto> {
    rpc.create_workflow(body).await.map_err(err)
}

#[tauri::command]
async fn enable_workflow(
    rpc: State<'_, TauriRpc>,
    workflow_id: Id<Workflow>,
) -> CmdResult<WorkflowDto> {
    rpc.enable_workflow(workflow_id).await.map_err(err)
}

#[tauri::command]
async fn disable_workflow(
    rpc: State<'_, TauriRpc>,
    workflow_id: Id<Workflow>,
) -> CmdResult<WorkflowDto> {
    rpc.disable_workflow(workflow_id).await.map_err(err)
}

#[tauri::command]
async fn fire_workflow(
    rpc: State<'_, TauriRpc>,
    workflow_id: Id<Workflow>,
    space_id: Id<Space>,
) -> CmdResult<RunOutcomeDto> {
    rpc.fire_workflow(workflow_id, space_id).await.map_err(err)
}

#[tauri::command]
async fn create_group(
    rpc: State<'_, TauriRpc>,
    workspace_id: Id<Workspace>,
    name: String,
) -> CmdResult<GroupDto> {
    rpc.create_group(workspace_id, &name).await.map_err(err)
}

#[tauri::command]
async fn add_group_member(
    rpc: State<'_, TauriRpc>,
    group_id: Id<Group>,
    identity_id: Id<Identity>,
) -> CmdResult<()> {
    rpc.add_group_member(group_id, identity_id)
        .await
        .map_err(err)
}

#[tauri::command]
async fn list_members(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
) -> CmdResult<Vec<lazyboy_wire::MembershipDto>> {
    rpc.list_members(space_id).await.map_err(err)
}

#[tauri::command]
async fn grant_membership(
    rpc: State<'_, TauriRpc>,
    space_id: Id<Space>,
    principal_kind: String,
    principal_id: String,
    role: String,
) -> CmdResult<CreatedIdDto> {
    rpc.grant_membership(space_id, &principal_kind, &principal_id, &role)
        .await
        .map_err(err)
}

/// Subscribe the webview to a space: spawn a poller that emits each newly
/// appended timeline message on the `space:{id}` event channel. The
/// desktop mirror of the HTTP shell's polling SSE bridge — same wire
/// shape (`MessageDto`), same half-second cadence, replaceable by a
/// store-side subscription when one lands. Returns once the poller is
/// spawned; the UI tears down by dropping its listener.
#[tauri::command]
fn subscribe(app: tauri::AppHandle, space_id: Id<Space>) {
    let rpc = app.state::<TauriRpc>().inner().clone();
    let channel = format!("space:{space_id}");
    tauri::async_runtime::spawn(async move {
        let mut cursor: Cursor = 0;
        loop {
            if let Ok((fresh, advanced)) = new_messages_since(rpc.store(), space_id, cursor).await {
                cursor = advanced;
                for dto in fresh {
                    if app.emit(&channel, &dto).is_err() {
                        return;
                    }
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
}

/// Build the desktop app: inject the in-process backend as managed state
/// and register the command surface. The single trust boundary is the
/// desktop session itself (SCOPE.md R4), so there is no bearer here — the
/// equivalent gate the HTTP shell carries for cross-origin clients.
pub fn run(rpc: TauriRpc) {
    // Bring goose up under the saved provider before the webview loads, so
    // the desktop shell behaves like the server's boot-time launch.
    {
        let rpc = rpc.clone();
        tauri::async_runtime::spawn(async move { rpc.start_goose().await });
    }
    tauri::Builder::default()
        .manage(rpc)
        .invoke_handler(tauri::generate_handler![
            list_spaces,
            create_space,
            timeline,
            list_pending,
            list_tasks,
            create_task,
            list_runs,
            start_run,
            decide,
            health,
            list_goose_providers,
            get_goose_config,
            set_goose_config,
            list_decisions,
            record_decision,
            list_reminders,
            create_reminder,
            dismiss_reminder,
            list_calendar,
            upsert_calendar,
            list_integrations,
            create_integration,
            ingress,
            set_feed_visibility,
            list_workflows,
            create_workflow,
            enable_workflow,
            disable_workflow,
            fire_workflow,
            create_group,
            add_group_member,
            list_members,
            grant_membership,
            subscribe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running lazyboy desktop shell");
}
