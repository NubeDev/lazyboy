//! Deterministic, network-free tests for the always-compiled sync
//! logic: the LWW merge decision, the outbox-to-wire mapping, the apply
//! decision, and event serde. The live-session paths are gated behind
//! the `zenoh` feature and are not exercised here, so default
//! `cargo test` needs no network.

use lazyboy_store::OutboxEventRow;
use lazyboy_sync::apply::{decide, ApplyAction};
use lazyboy_sync::drain::{key_for, to_publication};
use lazyboy_sync::event::SyncEvent;
use lazyboy_sync::merge::{incoming_wins, MergeKey};
use lazyboy_types::Id;
use time::{Duration, OffsetDateTime};

fn key(secs: i64, seq: i64) -> MergeKey {
    MergeKey {
        occurred_at: OffsetDateTime::UNIX_EPOCH + Duration::seconds(secs),
        seq,
    }
}

#[test]
fn lww_newer_timestamp_wins() {
    assert!(incoming_wins(key(10, 1), key(20, 1)));
    assert!(!incoming_wins(key(20, 1), key(10, 1)));
}

#[test]
fn lww_ties_break_on_higher_seq() {
    assert!(incoming_wins(key(10, 1), key(10, 2)));
    assert!(!incoming_wins(key(10, 2), key(10, 1)));
    assert!(!incoming_wins(key(10, 1), key(10, 1)));
}

#[test]
fn append_only_always_inserts() {
    let event = SyncEvent {
        aggregate: "message".into(),
        aggregate_id: "m1".into(),
        seq: 5,
        occurred_at: OffsetDateTime::UNIX_EPOCH,
        payload: serde_json::json!({}),
    };
    assert_eq!(decide(&event, None), ApplyAction::InsertIfAbsent);
    assert_eq!(
        decide(&event, Some(key(99, 99))),
        ApplyAction::InsertIfAbsent
    );
}

#[test]
fn mutable_row_uses_lww() {
    let event = SyncEvent {
        aggregate: "task".into(),
        aggregate_id: "t1".into(),
        seq: 2,
        occurred_at: OffsetDateTime::UNIX_EPOCH + Duration::seconds(50),
        payload: serde_json::json!({"state": "done"}),
    };
    assert_eq!(decide(&event, None), ApplyAction::Overwrite);
    assert_eq!(decide(&event, Some(key(10, 1))), ApplyAction::Overwrite);
    assert_eq!(decide(&event, Some(key(99, 1))), ApplyAction::Skip);
}

#[test]
fn drain_maps_row_to_publication() {
    let row = OutboxEventRow {
        id: Id::new(),
        aggregate: "task".into(),
        aggregate_id: "t1".into(),
        event_json: r#"{"op":"task.set_state","state":"done"}"#.into(),
        seq: 7,
        created_at: OffsetDateTime::UNIX_EPOCH + Duration::seconds(123),
        synced_at: None,
    };
    let pubn = to_publication("acme", &row).unwrap();
    assert_eq!(pubn.key, "lazyboy/acme/task/t1");

    let event: SyncEvent = serde_json::from_slice(&pubn.payload).unwrap();
    assert_eq!(event.aggregate, "task");
    assert_eq!(event.seq, 7);
    assert_eq!(event.payload["state"], "done");
}

#[test]
fn key_for_scopes_by_workspace() {
    assert_eq!(key_for("acme", "message", "m1"), "lazyboy/acme/message/m1");
}

#[test]
fn event_round_trips() {
    let event = SyncEvent {
        aggregate: "task".into(),
        aggregate_id: "t1".into(),
        seq: 3,
        occurred_at: OffsetDateTime::UNIX_EPOCH + Duration::seconds(9),
        payload: serde_json::json!({"state": "open", "n": 1}),
    };
    let bytes = serde_json::to_vec(&event).unwrap();
    let back: SyncEvent = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(event, back);
}
