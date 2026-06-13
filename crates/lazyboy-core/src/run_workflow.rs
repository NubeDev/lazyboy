use lazyboy_bridge::GooseClient;
use lazyboy_store::repo;
use lazyboy_types::domain::{ApprovalPolicy, ApprovalStatus, Space, Workflow};
use lazyboy_types::Id;

use crate::engine::Engine;
use crate::start_run::RunOutcome;
use crate::CoreError;

impl<G: GooseClient> Engine<G> {
    /// Fire a saved workflow into a space (SCOPE.md "Workflows and
    /// automation"). Opens an agent run for the workflow's prompt,
    /// records a `workflow_runs` row linking the two, and drives Goose
    /// under the workflow's approval policy.
    ///
    /// The load-bearing behaviour is the policy:
    ///
    /// - `require_approval`: identical to an interactive run. `start_run`
    ///   parks the first outside-world step as a pending `approvals` row
    ///   and returns `AwaitingApproval`; a human resolves it later.
    ///
    /// - `auto_approve`: the single sanctioned R6 exception. We do NOT
    ///   bypass the gate — the `approvals` row is still written first by
    ///   the drive loop (`import_update` -> `approval::request`), so the
    ///   audit invariant "what did the agent do and on whose authority"
    ///   holds. Only then does this path auto-resolve that same row
    ///   through the ordinary `resolve_approval` machinery (status
    ///   `approved`, `resolved_by` = the workflow's agent principal),
    ///   which answers Goose and continues driving. Write-then-resolve,
    ///   never write-skip.
    pub async fn run_workflow(
        &self,
        workflow_id: Id<Workflow>,
        space_id: Id<Space>,
    ) -> Result<RunOutcome, CoreError> {
        let workflow = repo::workflow::get(&self.store, workflow_id).await?;

        let started = self
            .start_run(space_id, &workflow.name, &workflow.steps_json)
            .await?;
        repo::workflow::record_run(&self.store, workflow_id, space_id, started.run_id).await?;

        let outcome = match workflow.approval_policy {
            ApprovalPolicy::RequireApproval => started.outcome,
            ApprovalPolicy::AutoApprove => {
                self.auto_resolve_to_end(started.run_id, started.outcome)
                    .await?
            }
        };
        Ok(outcome)
    }

    /// Drive an auto-approve run to its end. Each time a step parks an
    /// approval, the durable row is already written (audit, R6); resolve
    /// it as the agent principal through the normal path, which answers
    /// Goose and re-drives. Loops so a multi-step workflow clears each
    /// checkpoint in turn.
    async fn auto_resolve_to_end(
        &self,
        run_id: Id<lazyboy_types::domain::AgentRun>,
        mut outcome: RunOutcome,
    ) -> Result<RunOutcome, CoreError> {
        while outcome == RunOutcome::AwaitingApproval {
            let approval_id = repo::approval::pending_for_run(&self.store, run_id)
                .await?
                .ok_or_else(|| CoreError::NoPendingPermission(run_id.to_string()))?;
            match self
                .resolve_approval(approval_id, ApprovalStatus::Approved, self.agent_identity)
                .await?
            {
                Some(next) => outcome = next,
                None => break,
            }
        }
        Ok(outcome)
    }
}
