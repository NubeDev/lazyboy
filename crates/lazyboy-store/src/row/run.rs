use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{AgentRun, RunStatus, Space, Task};
use lazyboy_types::Id;

/// A decoded `agent_runs` row.
#[derive(Debug, Clone)]
pub struct RunRow {
    pub id: Id<AgentRun>,
    pub space_id: Id<Space>,
    pub task_id: Id<Task>,
    pub goose_session_id: Option<String>,
    pub status: RunStatus,
}

impl RunRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "agent_runs.id")?,
            space_id: decode::id(row.try_get("space_id")?, "agent_runs.space_id")?,
            task_id: decode::id(row.try_get("task_id")?, "agent_runs.task_id")?,
            goose_session_id: row.try_get("goose_session_id")?,
            status: decode::parse(row.try_get("status")?, "agent_runs.status")?,
        })
    }
}
