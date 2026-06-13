//! Drives the CLI command layer (the same functions the `lazyboy` binary
//! calls) against an in-process fake goose, over a real SQLite file. This
//! covers the wiring the unit/adapter tests do not: the sidecar `Config`,
//! `init`, and `run` reaching a gated tool through the host transport.
//!
//! Scope note: this stops at the parked approval. Resuming it
//! (`decide`) opens a *new* goose connection, and faithfully faking
//! goose replaying a suspended session across connections is more fake
//! than it is worth — the durable approve-resume primitive is already
//! proven against `FakeGoose` in `lazyboy-core`, and the single-process
//! approve-resume over the live transport in `lazyboy-adapters-host`.

#[path = "support/fake_acp_server.rs"]
mod fake_acp_server;

use fake_acp_server::FakeAcp;
use lazyboy_cli::{commands, Config};
use lazyboy_store::{repo, Store};

#[tokio::test]
async fn init_then_run_parks_an_approval_over_the_cli_layer() {
    let server = FakeAcp::start_gated().await;

    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let db = dir.join("cli_approval.db");
    let _ = std::fs::remove_file(&db);
    let _ = std::fs::remove_file(Config::path_for(db.to_str().unwrap()));
    let db_path = db.to_str().unwrap();

    let store = Store::connect(&format!("sqlite://{db_path}"))
        .await
        .unwrap();

    // init writes the sidecar; a fresh load must see the same ids.
    commands::init(&store, db_path, "demo").await.unwrap();
    let cfg = Config::load(db_path).unwrap();
    assert_eq!(Config::load(db_path).unwrap().space, cfg.space);

    // run gates on the tool and parks one durable approval.
    commands::run(&store, &cfg, &server.base, "list the files")
        .await
        .unwrap();

    let pending = repo::approval::list_pending(&store, cfg.space)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1, "the gated tool parked one approval");
    assert_eq!(pending[0].tool_name, "developer__shell");

    // The tool request reached the timeline for the operator to see.
    let bodies: Vec<_> = repo::message::list(&store, cfg.space)
        .await
        .unwrap()
        .into_iter()
        .map(|m| m.body)
        .collect();
    assert!(
        bodies.iter().any(|b| b.contains("developer__shell")),
        "tool request imported to the timeline: {bodies:?}"
    );
}
