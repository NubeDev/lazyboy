//! Exercises the replication outbox (SCOPE.md "Zenoh sync fabric")
//! against in-memory SQLite: per-aggregate monotonic seq, independent
//! seq per aggregate, the unsynced queue, and mark_synced.

use lazyboy_store::{repo, Store};
use time::OffsetDateTime;

fn ev<'a>(aggregate: &'a str, id: &'a str, body: &'a str) -> repo::outbox::NewOutboxEvent<'a> {
    repo::outbox::NewOutboxEvent {
        aggregate,
        aggregate_id: id,
        event_json: body,
    }
}

#[tokio::test]
async fn append_allocates_monotonic_per_aggregate_seq() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    repo::outbox::append(&store, ev("task", "t1", "{}"))
        .await
        .unwrap();
    repo::outbox::append(&store, ev("task", "t2", "{}"))
        .await
        .unwrap();
    repo::outbox::append(&store, ev("task", "t3", "{}"))
        .await
        .unwrap();

    let rows = repo::outbox::unsynced(&store).await.unwrap();
    let seqs: Vec<i64> = rows
        .iter()
        .filter(|r| r.aggregate == "task")
        .map(|r| r.seq)
        .collect();
    assert_eq!(seqs, vec![1, 2, 3]);
}

#[tokio::test]
async fn two_aggregates_have_independent_seq() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    repo::outbox::append(&store, ev("task", "t1", "{}"))
        .await
        .unwrap();
    repo::outbox::append(&store, ev("message", "m1", "{}"))
        .await
        .unwrap();
    repo::outbox::append(&store, ev("task", "t2", "{}"))
        .await
        .unwrap();
    repo::outbox::append(&store, ev("message", "m2", "{}"))
        .await
        .unwrap();

    let rows = repo::outbox::unsynced(&store).await.unwrap();
    let task: Vec<i64> = rows
        .iter()
        .filter(|r| r.aggregate == "task")
        .map(|r| r.seq)
        .collect();
    let message: Vec<i64> = rows
        .iter()
        .filter(|r| r.aggregate == "message")
        .map(|r| r.seq)
        .collect();
    assert_eq!(task, vec![1, 2]);
    assert_eq!(message, vec![1, 2]);
}

#[tokio::test]
async fn unsynced_returns_only_null_synced_in_order() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let first = repo::outbox::append(&store, ev("task", "t1", "{}"))
        .await
        .unwrap();
    repo::outbox::append(&store, ev("task", "t2", "{}"))
        .await
        .unwrap();

    repo::outbox::mark_synced(&store, first, OffsetDateTime::now_utc())
        .await
        .unwrap();

    let rows = repo::outbox::unsynced(&store).await.unwrap();
    let task: Vec<&str> = rows
        .iter()
        .filter(|r| r.aggregate == "task")
        .map(|r| r.aggregate_id.as_str())
        .collect();
    assert_eq!(task, vec!["t2"]);
}

#[tokio::test]
async fn mark_synced_flips_the_flag() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let id = repo::outbox::append(&store, ev("approval", "a1", "{}"))
        .await
        .unwrap();
    assert!(repo::outbox::unsynced(&store)
        .await
        .unwrap()
        .iter()
        .any(|r| r.aggregate == "approval"));

    repo::outbox::mark_synced(&store, id, OffsetDateTime::now_utc())
        .await
        .unwrap();
    assert!(!repo::outbox::unsynced(&store)
        .await
        .unwrap()
        .iter()
        .any(|r| r.aggregate == "approval"));
}
