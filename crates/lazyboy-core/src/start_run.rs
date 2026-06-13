use lazyboy_bridge::GooseClient;
use lazyboy_store::repo;
use lazyboy_types::domain::{AgentRun, Space, Task};
use lazyboy_types::Id;

use crate::drive::DriveStop;
use crate::engine::Engine;
use crate::CoreError;

/// The run a `start_run` kicked off and where it paused.
pub struct StartedRun {
    pub task_id: Id<Task>,
    pub run_id: Id<AgentRun>,
    pub outcome: RunOutcome,
}

/// Where the initial drive landed: blocked on an approval, or finished
/// the turn without needing one.
#[derive(Debug, PartialEq, Eq)]
pub enum RunOutcome {
    AwaitingApproval,
    Ended { succeeded: bool },
}

impl<G: GooseClient> Engine<G> {
    /// Turn a prompt in a space into work: open a task and run, open a
    /// Goose session, send the prompt, and drive until the first
    /// approval gate or the end of the turn ("talk becomes tasks,
    /// tasks become agent runs").
    pub async fn start_run(
        &self,
        space_id: Id<Space>,
        title: &str,
        prompt: &str,
    ) -> Result<StartedRun, CoreError> {
        let task_id = repo::task::create(&self.store, space_id, title, None).await?;
        let run_id = repo::run::create(&self.store, space_id, task_id).await?;
        repo::task::attach_run(&self.store, task_id, run_id).await?;

        let session = self.goose.new_session().await?;
        repo::run::set_session(&self.store, run_id, session.as_str()).await?;

        // Persist the prompt as the run's first event so a retry can
        // re-send the same prompt from the durable stream (SCOPE.md R1),
        // never an in-memory copy. The seq is drawn from the same
        // per-run counter drive() uses, so it occupies slot 1 and the
        // imported updates number from 2.
        repo::run::append_event(
            &self.store,
            repo::run::NewRunEvent {
                run_id,
                seq: self.next_seq(run_id),
                kind: "prompt",
                payload_json: prompt,
            },
        )
        .await?;

        self.goose.prompt(&session, prompt).await?;

        let outcome = match self.drive(run_id).await? {
            DriveStop::Approval => RunOutcome::AwaitingApproval,
            DriveStop::Ended { succeeded } => RunOutcome::Ended { succeeded },
            DriveStop::Drained => RunOutcome::Ended { succeeded: false },
        };
        Ok(StartedRun {
            task_id,
            run_id,
            outcome,
        })
    }
}
