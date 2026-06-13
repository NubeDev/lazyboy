use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{AgentRun, Approval, ApprovalStatus, Space};
use lazyboy_types::Id;

/// A decoded `approvals` row — Lazyboy's durable trust record.
#[derive(Debug, Clone)]
pub struct ApprovalRow {
    pub id: Id<Approval>,
    pub space_id: Id<Space>,
    pub agent_run_id: Id<AgentRun>,
    pub goose_session_id: String,
    pub tool_name: String,
    pub tool_input_json: String,
    pub status: ApprovalStatus,
}

impl ApprovalRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "approvals.id")?,
            space_id: decode::id(row.try_get("space_id")?, "approvals.space_id")?,
            agent_run_id: decode::id(row.try_get("agent_run_id")?, "approvals.agent_run_id")?,
            goose_session_id: row.try_get("goose_session_id")?,
            tool_name: row.try_get("tool_name")?,
            tool_input_json: row.try_get("tool_input_json")?,
            status: decode::parse(row.try_get("status")?, "approvals.status")?,
        })
    }
}
