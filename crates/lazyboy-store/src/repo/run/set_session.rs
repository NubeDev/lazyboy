use crate::{Store, StoreError};
use lazyboy_types::domain::AgentRun;
use lazyboy_types::Id;

/// Bind the Goose session id once the bridge has opened it. This is
/// the handle the crash-resume reconcile uses to `session/load`.
pub async fn set_session(
    store: &Store,
    run_id: Id<AgentRun>,
    goose_session_id: &str,
) -> Result<(), StoreError> {
    sqlx::query("UPDATE agent_runs SET goose_session_id = ? WHERE id = ?")
        .bind(goose_session_id)
        .bind(run_id.to_string())
        .execute(store.pool())
        .await?;
    Ok(())
}
