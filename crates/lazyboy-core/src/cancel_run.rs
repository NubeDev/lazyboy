use lazyboy_store::repo;
use lazyboy_types::domain::{AgentRun, Identity, RunStatus, TaskState};
use lazyboy_types::Id;

use lazyboy_bridge::GooseClient;

use crate::engine::Engine;
use crate::CoreError;

impl<G: GooseClient> Engine<G> {
    /// Stop a run: mark it `cancelled` and close any approval still
    /// parked on it by denying it, so no human is left resolving a tool
    /// request for a run that no longer advances (SCOPE.md build step 2
    /// "cancel"). The durable rows are the truth, so cancel is complete
    /// once they are written.
    ///
    /// Telling goose to abandon the session is deliberately not done
    /// here: the ACP seam Lazyboy models (GOOSE-ACP.md) has no cancel
    /// primitive, and a denied permission already releases goose's gated
    /// tool with a denial. Fabricating a cancel call would add transport
    /// surface the contract does not define.
    pub async fn cancel_run(
        &self,
        run_id: Id<AgentRun>,
        by: Id<Identity>,
    ) -> Result<(), CoreError> {
        let run = repo::run::get(&self.store, run_id).await?;
        repo::approval::deny_pending_for_run(&self.store, run_id, by).await?;
        repo::run::set_status(&self.store, run_id, RunStatus::Cancelled).await?;
        // A chat turn has no task; only a task-backed run cancels its task.
        if let Some(task_id) = run.task_id {
            repo::task::set_state(&self.store, task_id, TaskState::Cancelled).await?;
        }
        Ok(())
    }
}
