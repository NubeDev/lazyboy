//! The build-order step-1 vertical slice (SCOPE.md): one space, local,
//! with Goose, surviving restart through the crash-resume reconcile.
//! Driven against FakeGoose so the whole loop runs without a model.

use std::sync::Arc;

use lazyboy_bridge::{FakeGoose, PermissionRequest, SessionId, ToolCall, Update};
use lazyboy_core::{Engine, Reconciled, RunOutcome};
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{ApprovalStatus, MessageKind, RunStatus, Space, TaskState, Workspace};
use lazyboy_types::Id;

struct Fixture {
    store: Store,
    workspace: Id<Workspace>,
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
        workspace: ws,
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
        repo::task::get(&fx.store, started.task_id.unwrap())
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
        repo::task::get(&fx.store, started.task_id.unwrap())
            .await
            .unwrap()
            .state,
        TaskState::Done
    );
    // The timeline holds the agent text and the tool request (its approval
    // card), but not the raw tool result: that stays in the event log so
    // the channel is not filled with tool plumbing the agent narrates.
    let kinds: Vec<_> = repo::message::list(&fx.store, fx.space)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.kind)
        .collect();
    assert!(kinds.contains(&MessageKind::ToolRequest));
    assert!(
        !kinds.contains(&MessageKind::ToolResult),
        "tool results stay in the event log, not the timeline"
    );
    // The result is still recorded as a run event for audit.
    let events = repo::run::event_count(&fx.store, started.run_id)
        .await
        .unwrap();
    assert!(events > 0, "tool result recorded in the event log");
}

/// goose streams a turn as many `agent_message_chunk` updates; the driver
/// must coalesce a contiguous run of them into one timeline message, not
/// one per chunk (the bug where a sentence showed as a column of words).
#[tokio::test]
async fn streamed_agent_chunks_coalesce_into_one_message() {
    let fx = fixture().await;
    let goose = FakeGoose::new();
    goose.script(
        &SessionId("fake-sess-1".into()),
        vec![
            Update::AgentMessage { text: "Hello".into() },
            Update::AgentMessage { text: ", ".into() },
            Update::AgentMessage { text: "world".into() },
            Update::AgentMessage { text: "!".into() },
            Update::TurnEnded { stopped: true },
        ],
    );
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    engine.start_run(fx.space, "greet", "say hello").await.unwrap();

    let agent_msgs: Vec<_> = repo::message::list(&fx.store, fx.space)
        .await
        .unwrap()
        .into_iter()
        .filter(|m| m.kind == MessageKind::Agent)
        .collect();
    assert_eq!(
        agent_msgs.len(),
        1,
        "four streamed chunks must collapse to one timeline message"
    );
    assert_eq!(agent_msgs[0].body, "Hello, world!");
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

/// A run that asks to write a file, once approved, produces a `file`
/// artifact and an `artifact_ref` timeline message (SCOPE.md build
/// step 2 "artifacts imported").
fn artifact_script() -> Vec<Update> {
    vec![
        Update::PermissionRequested(PermissionRequest {
            request_id: "req-1".into(),
            tool: ToolCall {
                name: "developer__text_editor".into(),
                input_json: r#"{"command":"write","path":"out.txt"}"#.into(),
            },
        }),
        Update::ToolResult {
            tool_name: "developer__text_editor".into(),
            output_json: r#"{"path":"out.txt","bytes":12}"#.into(),
        },
        Update::TurnEnded { stopped: true },
    ]
}

#[tokio::test]
async fn tool_result_with_a_file_path_imports_an_artifact() {
    let fx = fixture().await;
    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), artifact_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let started = engine
        .start_run(fx.space, "write a file", "write out.txt")
        .await
        .unwrap();
    assert_eq!(started.outcome, RunOutcome::AwaitingApproval);

    let pending = repo::approval::list_pending(&fx.store, fx.space)
        .await
        .unwrap();
    engine
        .resolve_approval(pending[0].id, ApprovalStatus::Approved, fx.human)
        .await
        .unwrap();

    let artifacts = repo::artifact::list(&fx.store, fx.space).await.unwrap();
    assert_eq!(artifacts.len(), 1, "the written file landed as an artifact");
    assert_eq!(artifacts[0].kind, "file");
    assert_eq!(artifacts[0].uri, "out.txt");
    assert_eq!(artifacts[0].agent_run_id, Some(started.run_id));

    let kinds: Vec<_> = repo::message::list(&fx.store, fx.space)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.kind)
        .collect();
    assert!(
        kinds.contains(&MessageKind::ArtifactRef),
        "an artifact_ref message was appended"
    );
}

#[tokio::test]
async fn cancel_marks_the_run_cancelled_and_denies_its_approval() {
    let fx = fixture().await;
    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let started = engine
        .start_run(fx.space, "ship pricing", "list the files")
        .await
        .unwrap();
    assert_eq!(started.outcome, RunOutcome::AwaitingApproval);

    engine.cancel_run(started.run_id, fx.human).await.unwrap();

    assert_eq!(
        repo::run::get(&fx.store, started.run_id)
            .await
            .unwrap()
            .status,
        RunStatus::Cancelled
    );
    assert_eq!(
        repo::task::get(&fx.store, started.task_id.unwrap())
            .await
            .unwrap()
            .state,
        TaskState::Cancelled
    );
    assert!(
        repo::approval::list_pending(&fx.store, fx.space)
            .await
            .unwrap()
            .is_empty(),
        "the parked approval was closed by the cancel"
    );
}

#[tokio::test]
async fn retry_starts_a_fresh_run_for_the_same_task() {
    let fx = fixture().await;
    let goose = FakeGoose::new();
    // First run drives straight to a clean end; the retry plays the
    // next scripted session.
    goose.script(
        &SessionId("fake-sess-1".into()),
        vec![Update::TurnEnded { stopped: true }],
    );
    goose.script(
        &SessionId("fake-sess-2".into()),
        vec![
            Update::AgentMessage {
                text: "second attempt".into(),
            },
            Update::TurnEnded { stopped: true },
        ],
    );
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let first = engine
        .start_run(fx.space, "ship pricing", "do the thing")
        .await
        .unwrap();
    assert_eq!(first.outcome, RunOutcome::Ended { succeeded: true });

    let retried = engine.retry_run(first.run_id).await.unwrap();
    assert_eq!(retried.task_id, first.task_id, "same task, new run");
    assert_ne!(retried.run_id, first.run_id);
    assert_eq!(retried.outcome, RunOutcome::Ended { succeeded: true });

    // The retry re-sent the same prompt, read from the durable event.
    assert_eq!(
        repo::run::prompt_of(&fx.store, retried.run_id)
            .await
            .unwrap()
            .as_deref(),
        Some("do the thing")
    );
}

#[tokio::test]
async fn queue_lists_pending_approvals_across_the_workspace() {
    let fx = fixture().await;
    let second_space = repo::bootstrap::create_space(&fx.store, fx.workspace, "infra", "Infra")
        .await
        .unwrap();

    let goose = FakeGoose::new();
    goose.script(&SessionId("fake-sess-1".into()), approval_script());
    goose.script(&SessionId("fake-sess-2".into()), approval_script());
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    engine
        .start_run(fx.space, "a", "list the files")
        .await
        .unwrap();
    engine
        .start_run(second_space, "b", "list the files")
        .await
        .unwrap();

    let queue = repo::approval::queue(&fx.store, fx.workspace)
        .await
        .unwrap();
    assert_eq!(
        queue.len(),
        2,
        "both spaces' pending approvals are in the workspace queue"
    );
    // Per-space view still slices to one.
    assert_eq!(
        repo::approval::list_pending(&fx.store, fx.space)
            .await
            .unwrap()
            .len(),
        1
    );
}

/// A chat turn is a conversation, not a task: `start_chat` must drive the
/// agent without minting a task named after the message (the bug where
/// "make a task, pick up lenny at 1pm" became a task with that literal
/// title, and every chat line polluted the task list).
#[tokio::test]
async fn chat_turn_creates_no_task() {
    let fx = fixture().await;
    let goose = FakeGoose::new();
    goose.script(
        &SessionId("fake-sess-1".into()),
        vec![
            Update::AgentMessage {
                text: "On it.".into(),
            },
            Update::TurnEnded { stopped: true },
        ],
    );
    let engine = Engine::new(fx.store.clone(), goose, fx.agent);

    let started = engine
        .start_chat(fx.space, "make a new task, pick up lenny at 1pm")
        .await
        .unwrap();
    assert!(
        started.task_id.is_none(),
        "a chat turn must not create a task"
    );
    let tasks = repo::task::list(&fx.store, fx.space).await.unwrap();
    assert!(
        tasks.is_empty(),
        "chatting must not mint a task per message; got {tasks:?}"
    );
}

/// Each goose session is fresh, so a follow-up like "close it" only works
/// if the turn carries the recent conversation. Assert the second turn's
/// prompt to goose includes the prior exchange and the current message.
#[tokio::test]
async fn chat_carries_recent_context() {
    let fx = fixture().await;
    let goose = Arc::new(FakeGoose::new());
    goose.script(
        &SessionId("fake-sess-1".into()),
        vec![
            Update::AgentMessage {
                text: "Added it.".into(),
            },
            Update::TurnEnded { stopped: true },
        ],
    );
    goose.script(
        &SessionId("fake-sess-2".into()),
        vec![
            Update::AgentMessage {
                text: "Closed it.".into(),
            },
            Update::TurnEnded { stopped: true },
        ],
    );
    let engine = Engine::new(fx.store.clone(), goose.clone(), fx.agent);

    engine
        .start_chat(fx.space, "add a task to pick up Lenny")
        .await
        .unwrap();
    engine.start_chat(fx.space, "close it").await.unwrap();

    let prompts = goose.prompts();
    assert_eq!(prompts.len(), 2, "one prompt per turn");
    let second = &prompts[1].1;
    assert!(
        second.contains("pick up Lenny"),
        "second turn carries the prior user message:\n{second}"
    );
    assert!(
        second.contains("Added it."),
        "second turn carries the prior agent reply:\n{second}"
    );
    assert!(
        second.contains("close it"),
        "second turn carries the current message:\n{second}"
    );
}
