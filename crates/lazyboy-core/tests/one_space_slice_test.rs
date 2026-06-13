//! The build-order step-1 vertical slice (SCOPE.md): one space, local,
//! with Goose, surviving restart through the crash-resume reconcile.
//! Driven against FakeGoose so the whole loop runs without a model.

use std::sync::Arc;

use lazyboy_bridge::{FakeGoose, PermissionRequest, SessionId, ToolCall, Update};
use lazyboy_core::{Engine, Reconciled, RunOutcome};
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{ApprovalStatus, MessageKind, RunStatus, Space, TaskState};
use lazyboy_types::Id;

struct Fixture {
    store: Store,
    space: Id<Space>,
    agent: Id<lazyboy_types::domain::Identity>,
    human: Id<lazyboy_types::domain::Identity>,
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
    let human = repo::bootstrap::create_identity(
        &store,
        ws,
        repo::bootstrap::NewIdentity {
            kind: "human",
            display_name: "Ada",
            external_ref: None,
        },
    )
    .await
    .unwrap();
    Fixture {
        store,
        space,
        agent,
        human,
    }
}

/// The session goose will play for a run that asks to run one gated
/// shell tool, then finishes after it is allowed.
fn approval_script() -> Vec<Update> {
    vec![
        Update::AgentMessage {
            text: "I'll list the files.".into(),
        },
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
        Update::AgentMessage {
            text: "Done.".into(),
        },
        Update::TurnEnded { stopped: true },
    ]
}

#[tokio::test]
async fn run_blocks_on_approval_then_completes() {
    let fx = fixture().await;
    let goose = FakeGoose::new();
    // The session id FakeGoose hands out is deterministic: first new
    // session is fake-sess-1.
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let started = engine
        .start_run(fx.space, "ship pricing", "list the files")
        .await
        .unwrap();
    assert_eq!(started.outcome, RunOutcome::AwaitingApproval);

    let pending = repo::approval::list_pending(&fx.store, fx.space)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1, "the gated tool parked one approval");
    assert_eq!(
        repo::run::get(&fx.store, started.run_id)
            .await
            .unwrap()
            .status,
        RunStatus::WaitingApproval
    );
    assert_eq!(
        repo::task::get(&fx.store, started.task_id)
            .await
            .unwrap()
            .state,
        TaskState::BlockedOnApproval
    );

    let outcome = engine
        .resolve_approval(pending[0].id, ApprovalStatus::Approved, fx.human)
        .await
        .unwrap();
    assert_eq!(outcome, Some(RunOutcome::Ended { succeeded: true }));

    assert_eq!(
        repo::run::get(&fx.store, started.run_id)
            .await
            .unwrap()
            .status,
        RunStatus::Succeeded
    );
    assert_eq!(
        repo::task::get(&fx.store, started.task_id)
            .await
            .unwrap()
            .state,
        TaskState::Done
    );
    // The timeline holds agent text, the tool request, the tool result.
    let kinds: Vec<_> = repo::message::list(&fx.store, fx.space)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.kind)
        .collect();
    assert!(kinds.contains(&MessageKind::ToolRequest));
    assert!(kinds.contains(&MessageKind::ToolResult));
}

#[tokio::test]
async fn approval_row_survives_crash_and_resumes() {
    let fx = fixture().await;

    // First process: drive to the approval gate, then "crash" — drop
    // the engine and its in-memory request correlation.
    let started = {
        let goose = FakeGoose::new();
        goose.script(&SessionId("fake-sess-1".into()), approval_script());
        let engine = Engine::new(fx.store.clone(), goose, fx.agent);
        let started = engine
            .start_run(fx.space, "ship pricing", "list the files")
            .await
            .unwrap();
        assert_eq!(started.outcome, RunOutcome::AwaitingApproval);
        started
    };

    // The durable approval row is still there with no in-memory state
    // backing it — exactly the post-crash situation.
    let pending = repo::approval::list_pending(&fx.store, fx.space)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);

    // A human approves while the run is parked (before resume).
    assert!(
        repo::approval::resolve(&fx.store, pending[0].id, ApprovalStatus::Approved, fx.human)
            .await
            .unwrap()
    );

    // Second process: a fresh engine and goose. session/load replays
    // the history; the permission reappears with a fresh request id;
    // the already-recorded decision is re-sent.
    let goose = Arc::new(FakeGoose::new());
    goose.script(
        &SessionId("fake-sess-1".into()),
        vec![
            Update::PermissionRequested(PermissionRequest {
                request_id: "req-2-after-reload".into(),
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
        ],
    );
    let engine = Engine::new(fx.store.clone(), Arc::clone(&goose), fx.agent);

    let results = engine.reconcile().await.unwrap();
    assert_eq!(
        results,
        vec![Reconciled::DecisionReapplied { succeeded: true }]
    );

    assert!(
        goose.loaded_sessions().contains(&"fake-sess-1".to_string()),
        "reconcile re-attached the session via session/load"
    );
    assert_eq!(
        goose.answers().len(),
        1,
        "the recorded decision was re-sent once"
    );
    assert_eq!(
        repo::run::get(&fx.store, started.run_id)
            .await
            .unwrap()
            .status,
        RunStatus::Succeeded
    );
}
