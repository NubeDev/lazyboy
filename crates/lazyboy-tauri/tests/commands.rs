//! The pure command bodies of the desktop shell, exercised without the
//! tauri GUI stack (default features). The two mutating paths
//! (`start_run`, `decide`) drive the engine over the live goose transport
//! and are covered against `FakeGoose` in `lazyboy-core`; here we cover
//! the store-backed reads and the subscribe cursor the desktop poller
//! emits over its Tauri event channel.

use lazyboy_store::{repo, Store};
use lazyboy_tauri::{new_messages_since, TauriRpc};
use lazyboy_types::domain::MessageKind;

const GOOSE_URL: &str = "http://127.0.0.1:3284";

async fn store_with_space() -> (Store, lazyboy_types::Id<lazyboy_types::domain::Space>) {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let ws = repo::bootstrap::create_workspace(&store, "default")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, ws, "home", "Home")
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
    // Two starter messages so timeline and the cursor have rows to split.
    for body in ["hello", "world"] {
        repo::message::append(
            &store,
            repo::message::NewMessage {
                space_id: space,
                author: agent,
                kind: MessageKind::Human,
                body,
                ref_id: None,
            },
        )
        .await
        .unwrap();
    }
    (store, space)
}

#[tokio::test]
async fn list_spaces_returns_bootstrapped_space() {
    let (store, _space) = store_with_space().await;
    let rpc = TauriRpc::new(store, GOOSE_URL.to_owned());

    let spaces = rpc.list_spaces().await.unwrap();
    assert_eq!(spaces.len(), 1);
    assert_eq!(spaces[0].slug, "home");
    assert_eq!(spaces[0].title, "Home");
}

#[tokio::test]
async fn timeline_returns_appended_messages_in_order() {
    let (store, space) = store_with_space().await;
    let rpc = TauriRpc::new(store, GOOSE_URL.to_owned());

    let messages = rpc.timeline(space).await.unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].body, "hello");
    assert_eq!(messages[1].body, "world");
}

#[tokio::test]
async fn new_messages_since_emits_only_fresh_rows() {
    let (store, space) = store_with_space().await;

    // A fresh subscriber drains the whole backlog and the cursor lands
    // at the row count.
    let (fresh, cursor) = new_messages_since(&store, space, 0).await.unwrap();
    assert_eq!(fresh.len(), 2);
    assert_eq!(cursor, 2);

    // Caught up: the next poll with the advanced cursor yields nothing.
    let (none, cursor) = new_messages_since(&store, space, cursor).await.unwrap();
    assert!(none.is_empty());
    assert_eq!(cursor, 2);

    // A newly appended message is the only thing the next poll emits.
    let agent = repo::identity::find_by_kind(&store, "agent")
        .await
        .unwrap()
        .unwrap();
    repo::message::append(
        &store,
        repo::message::NewMessage {
            space_id: space,
            author: agent,
            kind: MessageKind::Agent,
            body: "fresh",
            ref_id: None,
        },
    )
    .await
    .unwrap();

    let (fresh, cursor) = new_messages_since(&store, space, cursor).await.unwrap();
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].body, "fresh");
    assert_eq!(cursor, 3);
}
