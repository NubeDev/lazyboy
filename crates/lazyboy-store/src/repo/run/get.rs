use crate::{RunRow, Store, StoreError};
use lazyboy_types::domain::AgentRun;
use lazyboy_types::Id;

pub async fn get(store: &Store, run_id: Id<AgentRun>) -> Result<RunRow, StoreError> {
    let row = sqlx::query("SELECT * FROM agent_runs WHERE id = ?")
        .bind(run_id.to_string())
        .fetch_optional(store.pool())
        .await?
        .ok_or_else(|| StoreError::NotFound(format!("agent_run {run_id}")))?;
    RunRow::from_row(&row)
}
