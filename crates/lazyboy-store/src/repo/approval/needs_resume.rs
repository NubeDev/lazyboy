use crate::{ApprovalRow, Store, StoreError};

/// The crash-resume reconcile query (SCOPE.md "Approvals and the
/// crash-resume seam"). Called once at startup, when no run has a live
/// in-process driver yet: every approval still `pending` or `approved`
/// is therefore one Goose may not have executed, and its run must be
/// `session/load`ed and re-driven to the approval point. A `denied`
/// approval is settled and skipped.
///
/// The status persisted on the run is deliberately not part of the
/// filter: a clean crash leaves the run last-written as `running` or
/// `waiting_approval` (it had no chance to write a terminal status), so
/// keying off "run is terminal" would miss exactly the case this query
/// exists to catch.
pub async fn needs_resume(store: &Store) -> Result<Vec<ApprovalRow>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM approvals WHERE status IN ('pending', 'approved') \
         ORDER BY requested_at",
    )
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(ApprovalRow::from_row).collect()
}
