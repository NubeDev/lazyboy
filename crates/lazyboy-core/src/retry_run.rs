use lazyboy_bridge::GooseClient;
use lazyboy_store::repo;
use lazyboy_types::domain::{AgentRun, TaskState};
use lazyboy_types::Id;

use crate::drive::DriveStop;
use crate::engine::Engine;
use crate::start_run::{RunOutcome, StartedRun};
use crate::CoreError;

impl<G: GooseClient> Engine<G> {
    /// Start a fresh run for the same task with the same prompt
    /// (SCOPE.md build step 2 "retry"). The original run is left as the
    /// historical record; retry opens a new Goose session and a new
    /// `agent_runs` row attached to the existing task, so the task's
    /// timeline shows both attempts.
    ///
    /// The prompt is read from the prior run's durable `prompt` event,
    /// not carried in memory, so a retry works in a fresh process after
    /// a crash just as the reconcile does.
    pub async fn retry_run(&self, run_id: Id<AgentRun>) -> Result<StartedRun, CoreError> {
        let prior = repo::run::get(&self.store, run_id).await?;
        let prompt = repo::run::prompt_of(&self.store, run_id)
            .await?
            .ok_or_else(|| CoreError::NoPrompt(run_id.to_string()))?;

        let task_id = prior.task_id;
        let new_run = repo::run::create(&self.store, prior.space_id, task_id).await?;
        repo::task::attach_run(&self.store, task_id, new_run).await?;
        repo::task::set_state(&self.store, task_id, TaskState::Running).await?;

        let session = self.goose.new_session().await?;
        repo::run::set_session(&self.store, new_run, session.as_str()).await?;
        repo::run::append_event(
            &self.store,
            repo::run::NewRunEvent {
                run_id: new_run,
                seq: self.next_seq(new_run),
                kind: "prompt",
                payload_json: &prompt,
            },
        )
        .await?;
        self.goose.prompt(&session, &prompt).await?;

        let outcome = match self.drive(new_run).await? {
            DriveStop::Approval => RunOutcome::AwaitingApproval,
            DriveStop::Ended { succeeded } => RunOutcome::Ended { succeeded },
            DriveStop::Drained => RunOutcome::Ended { succeeded: false },
        };
        Ok(StartedRun {
            task_id,
            run_id: new_run,
            outcome,
        })
    }
}
