use sqlx::Row;

use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, Approval};
use lazyboy_types::Id;

/// The pending approval parked for a run, if any. The auto-approve
/// workflow path uses this to find the row a step just parked so it can
/// resolve it through the normal machinery (the row is already written
/// for audit, R6).
pub async fn pending_for_run(
    store: &Store,
    run: Id<AgentRun>,
) -> Result<Option<Id<Approval>>, StoreError> {
    let row = sqlx::query(
        "SELECT id FROM approvals WHERE agent_run_id = ? AND status = 'pending' \
         ORDER BY requested_at LIMIT 1",
    )
    .bind(run.to_string())
    .fetch_optional(store.pool())
    .await?;
    match row {
        None => Ok(None),
        Some(row) => {
            let raw: String = row.try_get("id")?;
            uuid::Uuid::parse_str(&raw)
                .map(|u| Some(Id::from_uuid(u)))
                .map_err(|e| StoreError::Decode {
                    column: "approvals.id",
                    detail: e.to_string(),
                })
        }
    }
}
