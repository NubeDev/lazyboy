//! Exercises the durable-memory repos (SCOPE.md build step 4) against
//! in-memory SQLite: decisions record/list, reminders create/list/
//! set_status/due, and calendar upsert dedup + windowed list.

use lazyboy_store::{repo, Store};
use lazyboy_types::domain::ReminderStatus;
use time::{Duration, OffsetDateTime};

async fn seeded() -> (
    Store,
    lazyboy_types::Id<lazyboy_types::domain::Space>,
    lazyboy_types::Id<lazyboy_types::domain::Identity>,
) {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let ws = repo::bootstrap::create_workspace(&store, "acme")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, ws, "q3-migration", "Q3 migration")
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
async fn decision_records_and_lists() {
    let (store, space, human) = seeded().await;
    repo::decision::record(
        &store,
        repo::decision::NewDecision {
            space_id: space,
            message_id: None,
            summary: "ship behind a flag",
            decided_by_identity_id: Some(human),
        },
    )
    .await
    .unwrap();

    let rows = repo::decision::list(&store, space).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].summary, "ship behind a flag");
    assert_eq!(rows[0].decided_by_identity_id, Some(human));
}

#[tokio::test]
async fn reminder_create_list_set_status_and_due() {
    let (store, space, _human) = seeded().await;
    let now = OffsetDateTime::now_utc();
    let past = now - Duration::hours(1);
    let future = now + Duration::hours(1);

    let overdue = repo::reminder::create(
        &store,
        repo::reminder::NewReminder {
            space_id: space,
            task_id: None,
            due_at: past,
            body: "follow up",
        },
    )
    .await
    .unwrap();
    repo::reminder::create(
        &store,
        repo::reminder::NewReminder {
            space_id: space,
            task_id: None,
            due_at: future,
            body: "later",
        },
    )
    .await
    .unwrap();

    let listed = repo::reminder::list(&store, space).await.unwrap();
    assert_eq!(listed.len(), 2, "both reminders listed");
    assert_eq!(listed[0].body, "follow up", "soonest first");

    let due = repo::reminder::due(&store, now).await.unwrap();
    assert_eq!(due.len(), 1, "only the overdue reminder is due");
    assert_eq!(due[0].id, overdue);

    let fired = repo::reminder::set_status(&store, overdue, ReminderStatus::Fired)
        .await
        .unwrap();
    assert!(fired);
    let still_due = repo::reminder::due(&store, now).await.unwrap();
    assert!(still_due.is_empty(), "a fired reminder is no longer due");
}

#[tokio::test]
async fn calendar_upsert_dedups_on_external_ref() {
    let (store, space, _human) = seeded().await;
    let starts = OffsetDateTime::now_utc();

    let first = repo::calendar::upsert(
        &store,
        repo::calendar::NewCalendarEvent {
            space_id: space,
            source: "gcal",
            external_ref: Some("evt-1"),
            title: "kickoff",
            starts_at: starts,
            ends_at: None,
            meta_json: None,
        },
    )
    .await
    .unwrap();

    // A re-sync of the same external event refreshes the row in place.
    let second = repo::calendar::upsert(
        &store,
        repo::calendar::NewCalendarEvent {
            space_id: space,
            source: "gcal",
            external_ref: Some("evt-1"),
            title: "kickoff (rescheduled)",
            starts_at: starts + Duration::hours(2),
            ends_at: None,
            meta_json: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(first, second, "re-sync reuses the row, not a duplicate");

    let all = repo::calendar::list(&store, space, repo::calendar::Window::default())
        .await
        .unwrap();
    assert_eq!(all.len(), 1, "dedup keeps a single row");
    assert_eq!(all[0].title, "kickoff (rescheduled)", "update applied");
}

#[tokio::test]
async fn calendar_list_respects_window() {
    let (store, space, _human) = seeded().await;
    let base = OffsetDateTime::now_utc();
    for (i, ext) in ["a", "b", "c"].iter().enumerate() {
        repo::calendar::upsert(
            &store,
            repo::calendar::NewCalendarEvent {
                space_id: space,
                source: "gcal",
                external_ref: Some(ext),
                title: ext,
                starts_at: base + Duration::hours(i as i64),
                ends_at: None,
                meta_json: None,
            },
        )
        .await
        .unwrap();
    }

    let windowed = repo::calendar::list(
        &store,
        space,
        repo::calendar::Window {
            from: Some(base + Duration::minutes(30)),
            to: Some(base + Duration::hours(1) + Duration::minutes(30)),
        },
    )
    .await
    .unwrap();
    assert_eq!(windowed.len(), 1, "only the event inside the window");
    assert_eq!(windowed[0].title, "b");
}
