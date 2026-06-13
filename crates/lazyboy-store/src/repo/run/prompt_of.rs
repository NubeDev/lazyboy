use crate::{Store, StoreError};
use lazyboy_types::domain::AgentRun;
use lazyboy_types::Id;

/// The prompt a run was started with, recorded as its first
/// `agent_run_events` row (`kind = 'prompt'`). Retry re-sends this same
/// prompt for a fresh run, so the durable event stream — not an
/// in-memory copy — is what a retry reads.
pub async fn prompt_of(store: &Store, run_id: Id<AgentRun>) -> Result<Option<String>, StoreError> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT payload_json FROM agent_run_events \
         WHERE agent_run_id = ? AND kind = 'prompt' ORDER BY seq LIMIT 1",
    )
    .bind(run_id.to_string())
    .fetch_optional(store.pool())
    .await?;
    Ok(row.map(|r| r.get::<String, _>("payload_json")))
}
