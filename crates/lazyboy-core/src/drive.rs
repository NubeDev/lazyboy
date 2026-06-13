use lazyboy_bridge::{import_update, GooseClient, ImportContext, Imported};
use lazyboy_types::domain::{AgentRun, RunStatus, TaskState};
use lazyboy_types::Id;

use crate::engine::Engine;
use crate::CoreError;

/// Where a drive loop stopped.
pub(crate) enum DriveStop {
    /// Hit an approval gate; the run is parked in `waiting_approval`
    /// and the ACP request id is remembered for the resolve.
    Approval,
    /// The agent turn ended.
    Ended { succeeded: bool },
    /// The update stream ran dry without an explicit turn end (e.g.
    /// goose closed the session). Treated as a non-clean end.
    Drained,
}

impl<G: GooseClient> Engine<G> {
    /// Pull updates for a run's session and import each until an
    /// approval gate or the end of the turn. This is the one place the
    /// run lifecycle advances; start, resume, and post-decision
    /// continuation all funnel through it.
    pub(crate) async fn drive(&self, run_id: Id<AgentRun>) -> Result<DriveStop, CoreError> {
        let run = lazyboy_store::repo::run::get(&self.store, run_id).await?;
        let session = run
            .goose_session_id
            .clone()
            .ok_or_else(|| CoreError::NoPendingPermission(run_id.to_string()))?;
        let ctx = ImportContext {
            space_id: run.space_id,
            agent_run_id: run.id,
            goose_session_id: session.clone(),
            agent_identity: self.agent_identity,
        };
        let session = lazyboy_bridge::SessionId(session);

        lazyboy_store::repo::run::set_status(&self.store, run_id, RunStatus::Running).await?;

        loop {
            let Some(update) = self.goose.next_update(&session).await? else {
                self.end_run(run_id, false).await?;
                return Ok(DriveStop::Drained);
            };
            let seq = self.next_seq(run_id);
            match import_update(&self.store, &ctx, seq, &update).await? {
                Imported::Recorded => continue,
                Imported::AwaitingApproval {
                    approval_id,
                    request_id,
                } => {
                    self.remember_request(approval_id, request_id);
                    lazyboy_store::repo::run::set_status(
                        &self.store,
                        run_id,
                        RunStatus::WaitingApproval,
                    )
                    .await?;
                    self.set_task_state(run_id, TaskState::BlockedOnApproval)
                        .await?;
                    return Ok(DriveStop::Approval);
                }
                Imported::TurnEnded { succeeded } => {
                    self.end_run(run_id, succeeded).await?;
                    return Ok(DriveStop::Ended { succeeded });
                }
            }
        }
    }

    async fn end_run(&self, run_id: Id<AgentRun>, succeeded: bool) -> Result<(), CoreError> {
        let status = if succeeded {
            RunStatus::Succeeded
        } else {
            RunStatus::Failed
        };
        lazyboy_store::repo::run::set_status(&self.store, run_id, status).await?;
        let task_state = if succeeded {
            TaskState::Done
        } else {
            TaskState::Open
        };
        self.set_task_state(run_id, task_state).await?;
        Ok(())
    }

    async fn set_task_state(
        &self,
        run_id: Id<AgentRun>,
        state: TaskState,
    ) -> Result<(), CoreError> {
        let run = lazyboy_store::repo::run::get(&self.store, run_id).await?;
        lazyboy_store::repo::task::set_state(&self.store, run.task_id, state).await?;
        Ok(())
    }
}
