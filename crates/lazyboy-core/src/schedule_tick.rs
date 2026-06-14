use time::OffsetDateTime;

use lazyboy_bridge::GooseClient;
use lazyboy_store::repo;
use lazyboy_types::domain::{ScheduleTrigger, Workflow};
use lazyboy_types::Id;

use crate::engine::Engine;
use crate::start_run::RunOutcome;
use crate::CoreError;

impl<G: GooseClient> Engine<G> {
    /// The schedule half of the workflow agent (SCOPE.md "Workflows and
    /// automation", schedule trigger): given the previous tick instant
    /// `since` and now `at`, fire every enabled schedule-triggered
    /// workflow whose cron matches a minute in the half-open window
    /// `(since, at]`. Each firing drives Goose through `run_workflow`, so
    /// it parks the same durable `approvals` row and honours the same
    /// per-workflow approval policy as an interactive run (R6). The clock
    /// that supplies `since`/`at` is the host-side daemon; this function
    /// is pure selection-and-invocation, with no timer of its own, and so
    /// stays in the mobile-safe crate graph.
    ///
    /// The window form (not "is it due right now") is what makes a firing
    /// happen exactly once even when a tick is late or spans several
    /// matching minutes: the daemon advances `since` to the previous
    /// `at`, so each minute is considered in exactly one window.
    ///
    /// A schedule row whose `trigger_config_json` is absent or does not
    /// parse as a `ScheduleTrigger` is skipped, not fatal: one
    /// misconfigured workflow must not stop the node-wide clock from
    /// firing the others. Such rows are returned in `skipped` so the
    /// host can surface them.
    pub async fn dispatch_schedule_tick(
        &self,
        since: OffsetDateTime,
        at: OffsetDateTime,
    ) -> Result<ScheduleTickReport, CoreError> {
        let workflows = repo::workflow::enabled_schedules(&self.store).await?;
        let mut report = ScheduleTickReport::default();
        for wf in workflows {
            let trigger = match wf
                .trigger_config_json
                .as_deref()
                .and_then(|j| serde_json::from_str::<ScheduleTrigger>(j).ok())
            {
                Some(t) => t,
                None => {
                    report.skipped.push(wf.id);
                    continue;
                }
            };
            match trigger.fires_between(since, at) {
                Ok(true) => {
                    let outcome = self.run_workflow(wf.id, trigger.space_id).await?;
                    report.fired.push((wf.id, outcome));
                }
                Ok(false) => {}
                Err(_) => report.skipped.push(wf.id),
            }
        }
        Ok(report)
    }
}

/// What one schedule tick did: the workflows it fired (with their run
/// outcomes) and the schedule rows it skipped because their trigger
/// config was missing or unparseable.
#[derive(Debug, Default)]
pub struct ScheduleTickReport {
    pub fired: Vec<(Id<Workflow>, RunOutcome)>,
    pub skipped: Vec<Id<Workflow>>,
}
