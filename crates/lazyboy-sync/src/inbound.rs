//! Executing an apply decision against the local store (feature
//! `zenoh`). These writes deliberately bypass `repo::outbox::record`:
//! an applied remote change must not re-enter this node's outbox, or two
//! peers would echo each other's events forever. They go straight to the
//! tables through the shared pool, which is why they live here and not as
//! `repo` verbs (those all emit to the timeline's local-origin path).
//!
//! Coverage is intentionally scoped to the aggregates whose local writers
//! emit outbox events today (DOCS/ZENOH.md integration checklist):
//! `message` (append-only) and `task` (LWW). Each new wired mutation site
//! adds its inbound arm here.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use lazyboy_store::Store;

use crate::event::SyncEvent;
use crate::merge::MergeKey;
use crate::SyncError;

fn str_field<'a>(event: &'a SyncEvent, key: &str) -> Result<&'a str, SyncError> {
    event
        .payload
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| SyncError::Zenoh(format!("inbound event missing field {key}")))
}

/// The local row's LWW key for a mutable aggregate, or `None` if no row
/// exists yet. Only `task` is wired; an unknown mutable aggregate is
/// treated as absent so the inbound event takes effect.
pub async fn current_key(store: &Store, event: &SyncEvent) -> Result<Option<MergeKey>, SyncError> {
    if event.aggregate != "task" {
        return Ok(None);
    }
    let row: Option<(String,)> = sqlx::query_as("SELECT updated_at FROM tasks WHERE id = ?")
        .bind(&event.aggregate_id)
        .fetch_optional(store.pool())
        .await
        .map_err(lazyboy_store::StoreError::from)?;
    match row {
        None => Ok(None),
        Some((updated_at,)) => {
            let occurred_at = OffsetDateTime::parse(&updated_at, &Rfc3339)
                .map_err(|e| SyncError::Zenoh(format!("local task updated_at: {e}")))?;
            // The local row's seq is not stored per-row; for the tie-break
            // we use the incoming seq's predecessor semantics by treating
            // the local seq as the same value, so a strictly newer
            // occurred_at is required to overwrite within a millisecond.
            Ok(Some(MergeKey {
                occurred_at,
                seq: event.seq,
            }))
        }
    }
}

/// Idempotent insert for an append-only aggregate (messages). The id
/// comes from the originating node, so `INSERT OR IGNORE` makes a
/// redelivery a no-op and gives the union merge.
pub async fn insert_if_absent(store: &Store, event: &SyncEvent) -> Result<(), SyncError> {
    if event.aggregate != "message" {
        return Ok(());
    }
    let ref_id = event.payload.get("ref_id").and_then(|v| v.as_str());
    sqlx::query(
        "INSERT OR IGNORE INTO messages \
         (id, space_id, author_identity_id, kind, body, ts, ref_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(str_field(event, "id")?)
    .bind(str_field(event, "space_id")?)
    .bind(str_field(event, "author_identity_id")?)
    .bind(str_field(event, "kind")?)
    .bind(str_field(event, "body")?)
    .bind(event.occurred_at.format(&Rfc3339).expect("rfc3339 total"))
    .bind(ref_id)
    .execute(store.pool())
    .await
    .map_err(lazyboy_store::StoreError::from)?;
    Ok(())
}

/// Overwrite a mutable row (task state) that won LWW.
pub async fn overwrite(store: &Store, event: &SyncEvent) -> Result<(), SyncError> {
    if event.aggregate != "task" {
        return Ok(());
    }
    sqlx::query("UPDATE tasks SET state = ?, updated_at = ? WHERE id = ?")
        .bind(str_field(event, "state")?)
        .bind(str_field(event, "updated_at")?)
        .bind(&event.aggregate_id)
        .execute(store.pool())
        .await
        .map_err(lazyboy_store::StoreError::from)?;
    Ok(())
}
