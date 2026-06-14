//! The schedule-trigger half of the workflow agent (SCOPE.md "Workflows
//! and automation", schedule trigger). Driven against FakeGoose so the
//! whole gated loop runs without a model, matching the workflow slice
//! test. Asserts the window semantics, the space binding from the
//! trigger config, and that a firing goes through the same gated
//! `run_workflow` path (R6).

use lazyboy_bridge::{FakeGoose, PermissionRequest, SessionId, ToolCall, Update};
use lazyboy_core::{Engine, RunOutcome};
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{
    ApprovalPolicy, ApprovalStatus, RunStatus, Space, TriggerKind, Workflow, WorkflowStatus,
    Workspace,
};
use lazyboy_types::Id;
use time::macros::datetime;

struct Fixture {
    store: Store,
    workspace: Id<Workspace>,
    space: Id<Space>,
    agent: Id<lazyboy_types::domain::Identity>,
}

async fn fixture() -> Fixture {
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
    Fixture {
        store,
        workspace: ws,
        space,
        agent,
    }
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

async fn make_schedule(
    store: &Store,
    ws: Id<Workspace>,
    policy: ApprovalPolicy,
    trigger_config: Option<&str>,
    status: WorkflowStatus,
) -> Id<Workflow> {
    let id = repo::workflow::create(
        store,
        repo::workflow::NewWorkflow {
            workspace_id: ws,
            name: "nightly triage",
            trigger_kind: TriggerKind::Schedule,
            trigger_config_json: trigger_config,
            approval_policy: policy,
            steps_json: "list the files",
        },
    )
    .await
    .unwrap();
    if status == WorkflowStatus::Enabled {
        repo::workflow::set_status(store, id, WorkflowStatus::Enabled)
            .await
            .unwrap();
    }
    id
}

fn cron_config(cron: &str, space: Id<Space>) -> String {
    format!(r#"{{"cron":"{cron}","space_id":"{space}"}}"#)
}

#[tokio::test]
async fn fires_an_enabled_schedule_whose_cron_crosses_the_window() {
    let fx = fixture().await;
    let cfg = cron_config("0 9 * * *", fx.space);
    let wf = make_schedule(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some(&cfg),
        WorkflowStatus::Enabled,
    )
    .await;

    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    // A window that steps across 09:00 UTC.
    let report = engine
        .dispatch_schedule_tick(
            datetime!(2026-06-14 08:59 UTC),
            datetime!(2026-06-14 09:00 UTC),
        )
        .await
        .unwrap();

    assert_eq!(report.fired.len(), 1, "the due schedule fired");
    assert_eq!(report.fired[0].0, wf);
    assert_eq!(report.fired[0].1, RunOutcome::Ended { succeeded: true });
    assert!(report.skipped.is_empty());

    // It ran into the space named in the trigger config, through the
    // gated run_workflow path (R6 audit row written then resolved).
    let runs = repo::workflow::list_runs(&fx.store, wf).await.unwrap();
    assert_eq!(runs.len(), 1);
    let run = repo::run::get(&fx.store, runs[0].agent_run_id).await.unwrap();
    assert_eq!(run.status, RunStatus::Succeeded);
    let (status, resolved_by) = repo::approval::audit_of(&fx.store, runs[0].agent_run_id)
        .await
        .unwrap()
        .expect("the auto-approved step wrote an audit row");
    assert_eq!(status, ApprovalStatus::Approved);
    assert_eq!(resolved_by, Some(fx.agent));
}

#[tokio::test]
async fn does_not_fire_outside_the_window() {
    let fx = fixture().await;
    let cfg = cron_config("0 9 * * *", fx.space);
    make_schedule(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some(&cfg),
        WorkflowStatus::Enabled,
    )
    .await;

    let engine = Engine::new(fx.store.clone(), FakeGoose::new(), fx.agent);
    let report = engine
        .dispatch_schedule_tick(
            datetime!(2026-06-14 10:00 UTC),
            datetime!(2026-06-14 10:05 UTC),
        )
        .await
        .unwrap();
    assert!(report.fired.is_empty(), "09:00 is not in (10:00, 10:05]");
}

#[tokio::test]
async fn ignores_disabled_and_feed_workflows() {
    let fx = fixture().await;
    let cfg = cron_config("* * * * *", fx.space);

    // Disabled schedule: armed cron, but not an automation.
    make_schedule(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some(&cfg),
        WorkflowStatus::Disabled,
    )
    .await;

    // Enabled, but a feed trigger — not the schedule tick's concern.
    let feed = repo::workflow::create(
        &fx.store,
        repo::workflow::NewWorkflow {
            workspace_id: fx.workspace,
            name: "feed wf",
            trigger_kind: TriggerKind::Feed,
            trigger_config_json: Some(r#"{"repo":"acme/web"}"#),
            approval_policy: ApprovalPolicy::AutoApprove,
            steps_json: "x",
        },
    )
    .await
    .unwrap();
    repo::workflow::set_status(&fx.store, feed, WorkflowStatus::Enabled)
        .await
        .unwrap();

    let engine = Engine::new(fx.store.clone(), FakeGoose::new(), fx.agent);
    let report = engine
        .dispatch_schedule_tick(
            datetime!(2026-06-14 09:00 UTC),
            datetime!(2026-06-14 09:01 UTC),
        )
        .await
        .unwrap();
    assert!(report.fired.is_empty());
    assert!(report.skipped.is_empty());
}

#[tokio::test]
async fn skips_a_schedule_with_unparseable_config_without_aborting() {
    let fx = fixture().await;

    // A broken-config schedule and a good one, both due. The broken one
    // must be skipped, the good one must still fire.
    let broken = make_schedule(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some("not json"),
        WorkflowStatus::Enabled,
    )
    .await;
    let good_cfg = cron_config("* * * * *", fx.space);
    let good = make_schedule(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some(&good_cfg),
        WorkflowStatus::Enabled,
    )
    .await;

    let goose = FakeGoose::new();
    // `broken` is created first (ordered by created_at), so the good one
    // opens the first session.
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let report = engine
        .dispatch_schedule_tick(
            datetime!(2026-06-14 09:00 UTC),
            datetime!(2026-06-14 09:01 UTC),
        )
        .await
        .unwrap();

    assert_eq!(report.skipped, vec![broken]);
    assert_eq!(report.fired.len(), 1);
    assert_eq!(report.fired[0].0, good);
}
