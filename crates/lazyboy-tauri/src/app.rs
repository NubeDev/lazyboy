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

use lazyboy_types::domain::{Approval, ApprovalStatus, Space};
use lazyboy_types::Id;
use lazyboy_wire::{ApprovalDto, MessageDto, RunDto, RunOutcomeDto, SpaceDto, TaskDto};

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
    tauri::Builder::default()
        .manage(rpc)
        .invoke_handler(tauri::generate_handler![
            list_spaces,
            timeline,
            list_pending,
            list_tasks,
            list_runs,
            start_run,
            decide,
            subscribe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running lazyboy desktop shell");
}
