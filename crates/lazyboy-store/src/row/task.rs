use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{AgentRun, Space, Task, TaskState};
use lazyboy_types::Id;

/// A decoded `tasks` row.
#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: Id<Task>,
    pub space_id: Id<Space>,
    pub title: String,
    pub state: TaskState,
    pub agent_run_id: Option<Id<AgentRun>>,
}

impl TaskRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        let agent_run_id = row
            .try_get::<Option<String>, _>("agent_run_id")?
            .map(|v| decode::id(&v, "tasks.agent_run_id"))
            .transpose()?;
        Ok(Self {
            id: decode::id(row.try_get("id")?, "tasks.id")?,
            space_id: decode::id(row.try_get("space_id")?, "tasks.space_id")?,
            title: row.try_get("title")?,
            state: decode::parse(row.try_get("state")?, "tasks.state")?,
            agent_run_id,
        })
    }
}
