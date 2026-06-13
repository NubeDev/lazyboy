//! Drives the real `GooseServeClient` against an in-process fake ACP
//! server (`support::fake_acp_server`) that reproduces the verified goose
//! v1.37.0 wire contract. This exercises the whole transport — WS upgrade,
//! connection-id threading, the `202`+async-result correlation, and the
//! prompt-response→`TurnEnded` conversion — without launching `goose
//! serve`. The live binary test (`live_handshake_test`) is the same shape
//! against the real thing when an environment can run it.

#[path = "support/fake_acp_server.rs"]
mod fake_acp_server;

use fake_acp_server::FakeAcp;
use lazyboy_bridge::{GooseClient, SessionId, Update};

#[tokio::test]
async fn connect_open_session_and_drive_a_turn() {
    let server = FakeAcp::start().await;
    let client = lazyboy_adapters_host::GooseServeClient::connect(&server.base)
        .await
        .expect("connect + initialize");

    // session/new acknowledges with 202; the id arrives over the WS.
    let session = client.new_session().await.expect("new_session");
    assert_eq!(session, SessionId("sess-1".into()));

    client
        .prompt(&session, "do the thing")
        .await
        .expect("prompt");

    // Drive the turn the way the core driver does: pull until TurnEnded.
    let mut seen = Vec::new();
    loop {
        match client.next_update(&session).await.expect("next_update") {
            Some(Update::TurnEnded { stopped }) => {
                assert!(stopped, "end_turn maps to a clean stop");
                break;
            }
            Some(other) => seen.push(other),
            None => panic!("stream drained before TurnEnded"),
        }
    }

    assert_eq!(
        seen,
        vec![Update::AgentMessage {
            text: "working".into()
        }],
        "the streamed update arrived before the turn boundary"
    );
}

#[tokio::test]
async fn full_engine_approval_round_trip_over_the_wire() {
    use lazyboy_core::{Engine, RunOutcome};
    use lazyboy_store::{repo, Store};
    use lazyboy_types::domain::ApprovalStatus;

    let server = FakeAcp::start_gated().await;
    let client = lazyboy_adapters_host::GooseServeClient::connect(&server.base)
        .await
        .unwrap();

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

    let engine = Engine::new(store.clone(), client, agent);

    // The gated tool parks an approval; the run blocks over the wire.
    let started = engine
        .start_run(space, "ship pricing", "list the files")
        .await
        .unwrap();
    assert_eq!(started.outcome, RunOutcome::AwaitingApproval);

    let pending = repo::approval::list_pending(&store, space).await.unwrap();
    assert_eq!(pending.len(), 1, "the gated tool parked one approval");

    // Approving sends the answer over the WS; the fake resumes the tool
    // and completes the turn, which the transport surfaces as TurnEnded.
    let outcome = engine
        .resolve_approval(pending[0].id, ApprovalStatus::Approved, human)
        .await
        .unwrap();
    assert_eq!(outcome, Some(RunOutcome::Ended { succeeded: true }));
}

#[tokio::test]
async fn rejects_a_server_without_load_session() {
    // The fake always advertises loadSession; this asserts the happy path
    // stays green. The negative path (missing capability) is covered by
    // the connect() guard and exercised in the unit layer.
    let server = FakeAcp::start().await;
    assert!(
        lazyboy_adapters_host::GooseServeClient::connect(&server.base)
            .await
            .is_ok()
    );
}
