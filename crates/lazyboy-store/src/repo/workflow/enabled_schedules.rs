use crate::{Store, StoreError, WorkflowRow};

/// Every enabled, schedule-triggered workflow across all workspaces —
/// the candidate set the schedule tick evaluates against its clock. The
/// tick (host-side) decides which are actually due by parsing each row's
/// `trigger_config_json` cron; this query only narrows to armed
/// schedules so the tick never touches feed workflows or disabled ones.
///
/// Cross-workspace by design: the schedule tick is a single node-wide
/// clock, not a per-workspace call, mirroring `reminder::due`.
pub async fn enabled_schedules(store: &Store) -> Result<Vec<WorkflowRow>, StoreError> {
    let rows = sqlx::query(
        "SELECT * FROM workflows WHERE status = 'enabled' AND trigger_kind = 'schedule' \
         ORDER BY created_at, id",
    )
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(WorkflowRow::from_row).collect()
}
