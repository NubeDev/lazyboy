use sqlx::Row;

use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, ApprovalStatus, Identity};
use lazyboy_types::Id;

/// The status and resolver of the approval a run parked, if any. The
/// auto-approve workflow path uses this to prove the audit invariant
/// (R6): the row is WRITTEN then resolved by the agent principal, never
/// skipped. Returns `None` when the run parked no approval.
pub async fn audit_of(
    store: &Store,
    run: Id<AgentRun>,
) -> Result<Option<(ApprovalStatus, Option<Id<Identity>>)>, StoreError> {
    let row = sqlx::query(
        "SELECT status, resolved_by_identity_id FROM approvals WHERE agent_run_id = ? \
         ORDER BY requested_at LIMIT 1",
    )
    .bind(run.to_string())
    .fetch_optional(store.pool())
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let status_raw: String = row.try_get("status")?;
    let status: ApprovalStatus = status_raw.parse().map_err(|e: _| StoreError::Decode {
        column: "approvals.status",
        detail: format!("{e}"),
    })?;
    let resolved: Option<String> = row.try_get("resolved_by_identity_id")?;
    let resolved = match resolved {
        None => None,
        Some(raw) => Some(
            uuid::Uuid::parse_str(&raw)
                .map(Id::from_uuid)
                .map_err(|e| StoreError::Decode {
                    column: "approvals.resolved_by_identity_id",
                    detail: e.to_string(),
                })?,
        ),
    };
    Ok(Some((status, resolved)))
}
