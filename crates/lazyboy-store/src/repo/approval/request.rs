use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, Approval, ApprovalStatus, Space};
use lazyboy_types::Id;

/// The captured tool request. Written the moment Goose emits
/// `session/request_permission`, before anything is answered — this
/// row, not the runtime's in-memory oneshot, is what survives a crash.
pub struct NewApproval<'a> {
    pub space_id: Id<Space>,
    pub agent_run_id: Id<AgentRun>,
    pub goose_session_id: &'a str,
    pub tool_name: &'a str,
    pub tool_input_json: &'a str,
}

pub async fn request(store: &Store, new: NewApproval<'_>) -> Result<Id<Approval>, StoreError> {
    // A re-driven session after a crash replays the same tool request.
    // Reuse the still-pending approval for this run rather than minting
    // a duplicate, so the captured request stays the one row a human
    // resolves and the reconcile re-correlates to.
    if let Some(existing) = pending_for_run(store, new.agent_run_id).await? {
        return Ok(existing);
    }
    let id = Id::<Approval>::new();
    sqlx::query(
        "INSERT INTO approvals (id, space_id, agent_run_id, goose_session_id, tool_name, \
         tool_input_json, status, requested_at, resolved_at, resolved_by_identity_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL)",
    )
    .bind(id.to_string())
    .bind(new.space_id.to_string())
    .bind(new.agent_run_id.to_string())
    .bind(new.goose_session_id)
    .bind(new.tool_name)
    .bind(new.tool_input_json)
    .bind(ApprovalStatus::Pending.as_str())
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(id)
}

async fn pending_for_run(
    store: &Store,
    run: Id<AgentRun>,
) -> Result<Option<Id<Approval>>, StoreError> {
    use sqlx::Row;
    // pending or approved: a crash can leave an already-approved row
    // whose tool goose never executed, and the replay must re-bind to
    // it, not mint a new one (SCOPE.md crash-resume).
    let row = sqlx::query(
        "SELECT id FROM approvals WHERE agent_run_id = ? AND status IN ('pending', 'approved') \
         ORDER BY requested_at LIMIT 1",
    )
    .bind(run.to_string())
    .fetch_optional(store.pool())
    .await?;
    match row {
        None => Ok(None),
        Some(row) => {
            let id: String = row.try_get("id")?;
            uuid::Uuid::parse_str(&id)
                .map(|u| Some(Id::from_uuid(u)))
                .map_err(|e| StoreError::Decode {
                    column: "approvals.id",
                    detail: e.to_string(),
                })
        }
    }
}
