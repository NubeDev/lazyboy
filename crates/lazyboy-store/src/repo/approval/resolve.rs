use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{Approval, ApprovalStatus, Identity};
use lazyboy_types::Id;

/// Record a human decision on a pending approval. The guard on
/// `status = 'pending'` makes a double-resolve (two people clicking at
/// once) a no-op rather than letting the second overwrite the first.
/// Returns whether this call was the one that resolved it.
pub async fn resolve(
    store: &Store,
    approval_id: Id<Approval>,
    decision: ApprovalStatus,
    by: Id<Identity>,
) -> Result<bool, StoreError> {
    debug_assert!(matches!(
        decision,
        ApprovalStatus::Approved | ApprovalStatus::Denied
    ));
    let result = sqlx::query(
        "UPDATE approvals SET status = ?, resolved_at = ?, resolved_by_identity_id = ? \
         WHERE id = ? AND status = 'pending'",
    )
    .bind(decision.as_str())
    .bind(clock::fmt(clock::now()))
    .bind(by.to_string())
    .bind(approval_id.to_string())
    .execute(store.pool())
    .await?;
    Ok(result.rows_affected() > 0)
}
