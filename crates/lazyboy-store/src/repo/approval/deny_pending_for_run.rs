use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, ApprovalStatus, Identity};
use lazyboy_types::Id;

/// Close every still-pending approval on a run by denying it. Used when
/// a run is cancelled: a pending tool request must not be left dangling
/// for a human to resolve against a run that no longer exists. Returns
/// how many rows were closed. `approved` rows are left alone — they are
/// a decided action the crash-resume path may still need to apply.
pub async fn deny_pending_for_run(
    store: &Store,
    run_id: Id<AgentRun>,
    by: Id<Identity>,
) -> Result<u64, StoreError> {
    let result = sqlx::query(
        "UPDATE approvals SET status = ?, resolved_at = ?, resolved_by_identity_id = ? \
         WHERE agent_run_id = ? AND status = 'pending'",
    )
    .bind(ApprovalStatus::Denied.as_str())
    .bind(clock::fmt(clock::now()))
    .bind(by.to_string())
    .bind(run_id.to_string())
    .execute(store.pool())
    .await?;
    Ok(result.rows_affected())
}
