//! The host schedule clock, driven tick-by-tick against FakeGoose (no
//! real timer, no goose binary). Verifies the window advances across
//! ticks so a schedule fires exactly once, and that a firing goes
//! through the gated run_workflow path.

use std::time::Duration;

use lazyboy_adapters_host::Scheduler;
use lazyboy_bridge::{FakeGoose, PermissionRequest, SessionId, ToolCall, Update};
use lazyboy_core::Engine;
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{ApprovalPolicy, Space, TriggerKind, Workspace, WorkflowStatus};
use lazyboy_types::Id;
use time::macros::datetime;

async fn setup() -> (Store, Id<Workspace>, Id<Space>, Id<lazyboy_types::domain::Identity>) {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let ws = repo::bootstrap::create_workspace(&store, "acme")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, ws, "pricing", "Pricing")
        .await
        .unwrap();
    let agent = repo::bootstrap::create_identity(
        &store,
        ws,
        repo::bootstrap::NewIdentity {
            kind: "agent",
            display_name: "Goose",
            external_ref: None,
        },
    )
    .await
    .unwrap();
    (store, ws, space, agent)
}

fn approval_script() -> Vec<Update> {
    vec![
        Update::PermissionRequested(PermissionRequest {
            request_id: "req-1".into(),
            tool: ToolCall {
                name: "developer__shell".into(),
                input_json: r#"{"command":"ls"}"#.into(),
            },
        }),
        Update::ToolResult {
            tool_name: "developer__shell".into(),
            output_json: r#"{"stdout":"a.txt"}"#.into(),
        },
        Update::TurnEnded { stopped: true },
    ]
}

#[tokio::test]
async fn fires_once_then_advances_the_window() {
    let (store, ws, space, agent) = setup().await;
    let cfg = format!(r#"{{"cron":"0 9 * * *","space_id":"{space}"}}"#);
    let wf = repo::workflow::create(
        &store,
        repo::workflow::NewWorkflow {
            workspace_id: ws,
            name: "nightly",
            trigger_kind: TriggerKind::Schedule,
            trigger_config_json: Some(&cfg),
            approval_policy: ApprovalPolicy::AutoApprove,
            steps_json: "list files",
        },
    )
    .await
    .unwrap();
    repo::workflow::set_status(&store, wf, WorkflowStatus::Enabled)
        .await
        .unwrap();

    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(store.clone(), goose, agent);

    let mut scheduler = Scheduler::new(Duration::from_secs(60), datetime!(2026-06-14 08:58 UTC));

    // First tick crosses 09:00 -> fires once.
    let r1 = scheduler
        .tick_once(&engine, datetime!(2026-06-14 09:00 UTC))
        .await
        .unwrap();
    assert_eq!(r1.fired.len(), 1, "the schedule fired when its minute passed");
    assert_eq!(r1.fired[0].0, wf);

    // Second tick is a later window that does not re-include 09:00 -> no
    // re-fire, proving `since` advanced.
    let r2 = scheduler
        .tick_once(&engine, datetime!(2026-06-14 09:05 UTC))
        .await
        .unwrap();
    assert!(r2.fired.is_empty(), "the window advanced; 09:00 is behind it");

    // Exactly one agent run was recorded for the workflow.
    let runs = repo::workflow::list_runs(&store, wf).await.unwrap();
    assert_eq!(runs.len(), 1);
}

#[tokio::test]
async fn an_empty_first_window_fires_nothing() {
    let (store, ws, space, agent) = setup().await;
    let cfg = format!(r#"{{"cron":"* * * * *","space_id":"{space}"}}"#);
    let wf = repo::workflow::create(
        &store,
        repo::workflow::NewWorkflow {
            workspace_id: ws,
            name: "every minute",
            trigger_kind: TriggerKind::Schedule,
            trigger_config_json: Some(&cfg),
            approval_policy: ApprovalPolicy::AutoApprove,
            steps_json: "x",
        },
    )
    .await
    .unwrap();
    repo::workflow::set_status(&store, wf, WorkflowStatus::Enabled)
        .await
        .unwrap();

    let engine = Engine::new(store.clone(), FakeGoose::new(), agent);
    let start = datetime!(2026-06-14 09:00 UTC);
    let mut scheduler = Scheduler::new(Duration::from_secs(60), start);

    // A zero-width window (now == start): the immediate first tick fires
    // nothing, matching spawn()'s first immediate interval tick.
    let report = scheduler.tick_once(&engine, start).await.unwrap();
    assert!(report.fired.is_empty());
}
