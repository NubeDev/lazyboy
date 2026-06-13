use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{AgentRun, Space, Workflow, WorkflowRun};
use lazyboy_types::Id;

/// A decoded `workflow_runs` row — one firing of a workflow, linking it
/// to the `agent_run` it created so "what did this automation do" stays
/// answerable from the timeline (SCOPE.md).
#[derive(Debug, Clone)]
pub struct WorkflowRunRow {
    pub id: Id<WorkflowRun>,
    pub workflow_id: Id<Workflow>,
    pub space_id: Id<Space>,
    pub agent_run_id: Id<AgentRun>,
    pub status: String,
}

impl WorkflowRunRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "workflow_runs.id")?,
            workflow_id: decode::id(row.try_get("workflow_id")?, "workflow_runs.workflow_id")?,
            space_id: decode::id(row.try_get("space_id")?, "workflow_runs.space_id")?,
            agent_run_id: decode::id(row.try_get("agent_run_id")?, "workflow_runs.agent_run_id")?,
            status: row.try_get("status")?,
        })
    }
}
