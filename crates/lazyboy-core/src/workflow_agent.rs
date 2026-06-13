use lazyboy_bridge::GooseClient;
use lazyboy_store::repo;
use lazyboy_types::domain::{Space, TriggerKind, Workflow, WorkflowStatus, Workspace};
use lazyboy_types::Id;

use crate::engine::Engine;
use crate::start_run::RunOutcome;
use crate::CoreError;

/// A feed/ingress event the workflow agent watches, reduced to what
/// selection needs: which workspace it belongs to, the space it routes
/// into, and a match key. The match key is compared against each
/// enabled feed-triggered workflow's `trigger_config_json` so a workflow
/// fires only for the events it subscribed to.
pub struct FeedEvent<'a> {
    pub workspace_id: Id<Workspace>,
    pub space_id: Id<Space>,
    /// The `trigger_config_json` an enabled feed workflow must carry to
    /// match this event. Equality, not pattern logic: richer matching is
    /// a documented later concern (DOCS/WORKFLOWS.md).
    pub trigger_config_json: &'a str,
}

impl<G: GooseClient> Engine<G> {
    /// The workflow agent (SCOPE.md "Workflows and automation"): given a
    /// feed event, select the enabled feed-triggered workflows whose
    /// trigger config matches and fire each. It DRIVES Goose through
    /// `run_workflow` — every step is still a Goose tool call (R3); it
    /// does not replace the agent loop.
    ///
    /// This is the selection-and-invocation model, deliberately
    /// synchronous and event-driven. The live scheduler/feed-watcher
    /// daemon that arms triggers and delivers these events is the
    /// host-side integration point, not this function (DOCS/WORKFLOWS.md).
    pub async fn dispatch_feed_event(
        &self,
        event: &FeedEvent<'_>,
    ) -> Result<Vec<(Id<Workflow>, RunOutcome)>, CoreError> {
        let workflows = repo::workflow::list(&self.store, event.workspace_id).await?;
        let mut fired = Vec::new();
        for wf in workflows {
            if !Self::matches(&wf, event) {
                continue;
            }
            let outcome = self.run_workflow(wf.id, event.space_id).await?;
            fired.push((wf.id, outcome));
        }
        Ok(fired)
    }

    /// A workflow fires for a feed event when it is enabled (an
    /// automation), feed-triggered, and its trigger config equals the
    /// event's match key.
    fn matches(wf: &lazyboy_store::WorkflowRow, event: &FeedEvent<'_>) -> bool {
        wf.status == WorkflowStatus::Enabled
            && wf.trigger_kind == TriggerKind::Feed
            && wf.trigger_config_json.as_deref() == Some(event.trigger_config_json)
    }
}
