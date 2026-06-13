use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::AgentRun;
use lazyboy_types::Id;

/// One imported run event. `seq` is the per-run monotonic order the
/// bridge assigns as it reads the Goose stream; the (run, seq) unique
/// index makes a redelivered event a no-op rather than a duplicate.
pub struct NewRunEvent<'a> {
    pub run_id: Id<AgentRun>,
    pub seq: i64,
    pub kind: &'a str,
    pub payload_json: &'a str,
}

/// Append an imported event. Returns whether the row was new (false
/// means the seq already existed and the insert was ignored).
pub async fn append_event(store: &Store, ev: NewRunEvent<'_>) -> Result<bool, StoreError> {
    let id = uuid::Uuid::new_v4();
    let result = sqlx::query(
        "INSERT OR IGNORE INTO agent_run_events (id, agent_run_id, seq, kind, payload_json, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(ev.run_id.to_string())
    .bind(ev.seq)
    .bind(ev.kind)
    .bind(ev.payload_json)
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(result.rows_affected() > 0)
}
