//! The workflow-firing path under both approval policies (SCOPE.md
//! "Workflows and automation", R6). Driven against FakeGoose so the
//! whole loop runs without a model, matching the step-1 slice tests.

use lazyboy_bridge::{FakeGoose, PermissionRequest, SessionId, ToolCall, Update};
use lazyboy_core::{Engine, FeedEvent, RunOutcome};
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{
    ApprovalPolicy, ApprovalStatus, RunStatus, Space, TriggerKind, Workflow, WorkflowStatus,
    Workspace,
};
use lazyboy_types::Id;

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

/// A session that asks to run one gated shell tool, then finishes.
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

async fn make_workflow(
    store: &Store,
    ws: Id<Workspace>,
    policy: ApprovalPolicy,
    trigger_config: Option<&str>,
) -> Id<Workflow> {
    repo::workflow::create(
        store,
        repo::workflow::NewWorkflow {
            workspace_id: ws,
            name: "nightly triage",
            trigger_kind: TriggerKind::Feed,
            trigger_config_json: trigger_config,
            approval_policy: policy,
            steps_json: "list the files",
        },
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn require_approval_workflow_parks_a_pending_approval() {
    let fx = fixture().await;
    let workflow = make_workflow(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::RequireApproval,
        None,
    )
    .await;

    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let outcome = engine.run_workflow(workflow, fx.space).await.unwrap();
    assert_eq!(
        outcome,
        RunOutcome::AwaitingApproval,
        "require_approval parks exactly like an interactive run"
    );

    let pending = repo::approval::list_pending(&fx.store, fx.space)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1, "the gated step parked one approval");
    assert_eq!(pending[0].status, ApprovalStatus::Pending);

    // The firing was recorded against an agent run.
    let runs = repo::workflow::list_runs(&fx.store, workflow)
        .await
        .unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(
        repo::run::get(&fx.store, runs[0].agent_run_id)
            .await
            .unwrap()
            .status,
        RunStatus::WaitingApproval
    );
}

#[tokio::test]
async fn auto_approve_workflow_writes_then_resolves_and_completes() {
    let fx = fixture().await;
    let workflow = make_workflow(&fx.store, fx.workspace, ApprovalPolicy::AutoApprove, None).await;

    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let outcome = engine.run_workflow(workflow, fx.space).await.unwrap();
    assert_eq!(
        outcome,
        RunOutcome::Ended { succeeded: true },
        "auto_approve drives to the end without parking"
    );

    // Nothing is left pending.
    assert!(repo::approval::list_pending(&fx.store, fx.space)
        .await
        .unwrap()
        .is_empty());

    // The audit invariant (R6): the row was still WRITTEN — it now sits
    // approved with the agent principal as resolver, not skipped.
    let runs = repo::workflow::list_runs(&fx.store, workflow)
        .await
        .unwrap();
    let run_id = runs[0].agent_run_id;
    assert_eq!(
        repo::run::get(&fx.store, run_id).await.unwrap().status,
        RunStatus::Succeeded,
        "the run reached TurnEnded"
    );
    let queue = repo::approval::queue(&fx.store, fx.workspace)
        .await
        .unwrap();
    assert!(queue.is_empty(), "no approval is left pending in the queue");

    // The audit row exists, auto-resolved approved by the agent
    // principal (write-then-resolve, R6 audit — never skipped).
    let (status, resolved_by) = repo::approval::audit_of(&fx.store, run_id)
        .await
        .unwrap()
        .expect("the auto-approved step wrote an audit row");
    assert_eq!(status, ApprovalStatus::Approved);
    assert_eq!(
        resolved_by,
        Some(fx.agent),
        "resolved_by is the workflow's agent principal"
    );
}

#[tokio::test]
async fn workflow_agent_selects_matching_enabled_workflows() {
    let fx = fixture().await;
    let trigger = r#"{"repo":"acme/web"}"#;

    // One matching, enabled, auto-approve workflow; one disabled; one
    // with a non-matching trigger config.
    let matching = make_workflow(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some(trigger),
    )
    .await;
    repo::workflow::set_status(&fx.store, matching, WorkflowStatus::Enabled)
        .await
        .unwrap();

    let disabled = make_workflow(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some(trigger),
    )
    .await;
    let _ = disabled; // left disabled

    let other = make_workflow(
        &fx.store,
        fx.workspace,
        ApprovalPolicy::AutoApprove,
        Some(r#"{"repo":"acme/other"}"#),
    )
    .await;
    repo::workflow::set_status(&fx.store, other, WorkflowStatus::Enabled)
        .await
        .unwrap();

    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let event = FeedEvent {
        workspace_id: fx.workspace,
        space_id: fx.space,
        trigger_config_json: trigger,
    };
    let fired = engine.dispatch_feed_event(&event).await.unwrap();
    assert_eq!(fired.len(), 1, "only the matching enabled workflow fired");
    assert_eq!(fired[0].0, matching);
    assert_eq!(fired[0].1, RunOutcome::Ended { succeeded: true });
}
