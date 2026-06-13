use lazyboy_bridge::{Decision, GooseClient, SessionId};
use lazyboy_store::repo;
use lazyboy_types::domain::{Approval, ApprovalStatus, Identity};
use lazyboy_types::Id;

use crate::drive::DriveStop;
use crate::engine::Engine;
use crate::start_run::RunOutcome;
use crate::CoreError;

impl<G: GooseClient> Engine<G> {
    /// Apply a human decision to a pending approval and let the run
    /// continue. Persisting the decision comes first: if the process
    /// dies before goose is told, the reconcile re-drives and re-sends
    /// from the durable `approved`/`denied` row (SCOPE.md crash-resume).
    ///
    /// Returns where the run next paused, or `None` if the approval was
    /// already resolved by someone else (single-tenant, but two clients
    /// can still race) — in which case this call is a no-op.
    pub async fn resolve_approval(
        &self,
        approval_id: Id<Approval>,
        decision: ApprovalStatus,
        by: Id<Identity>,
    ) -> Result<Option<RunOutcome>, CoreError> {
        let bridge_decision = match Decision::from_status(decision) {
            Some(d) => d,
            None => return Ok(None),
        };

        if !repo::approval::resolve(&self.store, approval_id, decision, by).await? {
            return Ok(None);
        }

        let approval = repo::approval::get(&self.store, approval_id).await?;
        let request_id = self
            .take_request(approval_id)
            .ok_or_else(|| CoreError::NoPendingPermission(approval.agent_run_id.to_string()))?;

        let session = SessionId(approval.goose_session_id.clone());
        self.goose
            .answer_permission(&session, &request_id, bridge_decision)
            .await?;

        let outcome = match self.drive(approval.agent_run_id).await? {
            DriveStop::Approval => RunOutcome::AwaitingApproval,
            DriveStop::Ended { succeeded } => RunOutcome::Ended { succeeded },
            DriveStop::Drained => RunOutcome::Ended { succeeded: false },
        };
        Ok(Some(outcome))
    }
}
