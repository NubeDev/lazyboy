//! Exercises a full step-1 timeline against in-memory SQLite: bootstrap
//! -> message -> task -> run -> approval request -> resolve, plus the
//! crash-resume reconcile query.

use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{ApprovalStatus, MessageKind, RunStatus, TaskState};

async fn seeded() -> (
    Store,
    lazyboy_types::Id<lazyboy_types::domain::Space>,
    lazyboy_types::Id<lazyboy_types::domain::Identity>,
) {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let ws = repo::bootstrap::create_workspace(&store, "acme")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, ws, "new-pricing-page", "New pricing page")
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
    (store, space, human)
}

#[tokio::test]
async fn message_appends_and_lists_in_order() {
    let (store, space, human) = seeded().await;
    for body in ["first", "second", "third"] {
        repo::message::append(
            &store,
            repo::message::NewMessage {
                space_id: space,
                author: human,
                kind: MessageKind::Human,
                body,
                ref_id: None,
            },
        )
        .await
        .unwrap();
    }
    let msgs = repo::message::list(&store, space).await.unwrap();
    let bodies: Vec<_> = msgs.iter().map(|m| m.body.as_str()).collect();
    assert_eq!(bodies, ["first", "second", "third"]);
}

#[tokio::test]
async fn approval_request_resolve_round_trips() {
    let (store, space, human) = seeded().await;
    let task = repo::task::create(&store, space, "ship pricing", None)
        .await
        .unwrap();
    let run = repo::run::create(&store, space, task).await.unwrap();
    repo::task::attach_run(&store, task, run).await.unwrap();
    repo::run::set_session(&store, run, "goose-sess-1")
        .await
        .unwrap();
    repo::run::set_status(&store, run, RunStatus::WaitingApproval)
        .await
        .unwrap();
    repo::task::set_state(&store, task, TaskState::BlockedOnApproval)
        .await
        .unwrap();

    let approval = repo::approval::request(
        &store,
        repo::approval::NewApproval {
            space_id: space,
            agent_run_id: run,
            goose_session_id: "goose-sess-1",
            tool_name: "developer__shell",
            tool_input_json: r#"{"command":"ls"}"#,
        },
    )
    .await
    .unwrap();

    let pending = repo::approval::list_pending(&store, space).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].tool_name, "developer__shell");

    let first = repo::approval::resolve(&store, approval, ApprovalStatus::Approved, human)
        .await
        .unwrap();
    assert!(first, "first resolve wins");
    let second = repo::approval::resolve(&store, approval, ApprovalStatus::Denied, human)
        .await
        .unwrap();
    assert!(
        !second,
        "second resolve is a no-op on an already-resolved row"
    );

    let row = repo::approval::get(&store, approval).await.unwrap();
    assert_eq!(row.status, ApprovalStatus::Approved);
    assert!(repo::approval::list_pending(&store, space)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn needs_resume_finds_unexecuted_approval() {
    let (store, space, _human) = seeded().await;
    let task = repo::task::create(&store, space, "t", None).await.unwrap();
    let run = repo::run::create(&store, space, task).await.unwrap();
    repo::run::set_session(&store, run, "sess").await.unwrap();
    repo::run::set_status(&store, run, RunStatus::WaitingApproval)
        .await
        .unwrap();
    repo::approval::request(
        &store,
        repo::approval::NewApproval {
            space_id: space,
            agent_run_id: run,
            goose_session_id: "sess",
            tool_name: "developer__shell",
            tool_input_json: "{}",
        },
    )
    .await
    .unwrap();

    let resume = repo::approval::needs_resume(&store).await.unwrap();
    assert_eq!(resume.len(), 1, "a pending approval is a resume candidate");
    assert_eq!(resume[0].goose_session_id, "sess");
}

#[tokio::test]
async fn run_event_import_is_idempotent() {
    let (store, space, _human) = seeded().await;
    let task = repo::task::create(&store, space, "t", None).await.unwrap();
    let run = repo::run::create(&store, space, task).await.unwrap();
    let ev = || repo::run::NewRunEvent {
        run_id: run,
        seq: 7,
        kind: "tool_call",
        payload_json: "{}",
    };
    assert!(
        repo::run::append_event(&store, ev()).await.unwrap(),
        "first insert is new"
    );
    assert!(
        !repo::run::append_event(&store, ev()).await.unwrap(),
        "redelivered seq is ignored"
    );
}

/// Reopening a persistent database must be a no-op, not a "table already
/// exists" error: the step-1 restart path connects to the same file
/// twice. Memory dbs cannot catch this (each connect is fresh), so this
/// uses a real file under the target dir.
#[tokio::test]
async fn reopening_a_file_db_is_idempotent() {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("reopen.db");
    let _ = std::fs::remove_file(&path);
    let url = format!("sqlite://{}", path.display());

    let first = Store::connect(&url).await.unwrap();
    let ws = repo::bootstrap::create_workspace(&first, "acme")
        .await
        .unwrap();
    drop(first);

    // Second connect re-runs migrate over the existing schema.
    let second = Store::connect(&url).await.unwrap();
    repo::bootstrap::create_space(&second, ws, "s", "S")
        .await
        .expect("schema survived reopen and is writable");
}
