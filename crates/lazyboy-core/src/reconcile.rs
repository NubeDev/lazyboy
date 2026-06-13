use lazyboy_bridge::{Decision, GooseClient, SessionId};
use lazyboy_store::{repo, ApprovalRow};
use lazyboy_types::domain::ApprovalStatus;

use crate::drive::DriveStop;
use crate::engine::Engine;
use crate::CoreError;

/// Outcome of reconciling one in-flight approval after a restart.
#[derive(Debug, PartialEq, Eq)]
pub enum Reconciled {
    /// The approval was already decided before the crash; the decision
    /// was re-sent to goose and the run driven on.
    DecisionReapplied { succeeded: bool },
    /// The approval is still pending; its session was re-attached and
    /// re-driven to the gate, so a fresh `resolve_approval` now works.
    Reparked,
}

impl<G: GooseClient> Engine<G> {
    /// Re-establish every in-flight approval after a crash (SCOPE.md
    /// "Approvals and the crash-resume seam"). The durable rows are the
    /// truth; goose only has to resume the run. For each candidate we
    /// `session/load`, re-drive to the approval point to recover the
    /// fresh ACP request id, then re-apply any decision already taken.
    ///
    /// Re-driving replays goose's persisted history; the (run, seq)
    /// unique index makes event re-import a no-op, but timeline message
    /// idempotency on replay is the deferred determinism question
    /// (SCOPE.md open question 2), confirmed in the bridge phase.
    pub async fn reconcile(&self) -> Result<Vec<Reconciled>, CoreError> {
        let candidates = repo::approval::needs_resume(&self.store).await?;
        let mut results = Vec::with_capacity(candidates.len());
        for approval in candidates {
            results.push(self.reconcile_one(approval).await?);
        }
        Ok(results)
    }

    async fn reconcile_one(&self, approval: ApprovalRow) -> Result<Reconciled, CoreError> {
        let session = SessionId(approval.goose_session_id.clone());
        self.goose.load_session(&session).await?;
        self.reseed_seq(approval.agent_run_id).await?;

        // Re-drive to the gate. The replayed PermissionRequested lands a
        // fresh request id, which drive() stores via remember_request,
        // re-correlating this approval for a decision send.
        let stop = self.drive(approval.agent_run_id).await?;

        match Decision::from_status(approval.status) {
            None => {
                debug_assert_eq!(approval.status, ApprovalStatus::Pending);
                let _ = stop;
                Ok(Reconciled::Reparked)
            }
            Some(decision) => {
                let request_id = self.take_request(approval.id).ok_or_else(|| {
                    CoreError::NoPendingPermission(approval.agent_run_id.to_string())
                })?;
                self.goose
                    .answer_permission(&session, &request_id, decision)
                    .await?;
                match self.drive(approval.agent_run_id).await? {
                    DriveStop::Ended { succeeded } => {
                        Ok(Reconciled::DecisionReapplied { succeeded })
                    }
                    DriveStop::Approval | DriveStop::Drained => {
                        Ok(Reconciled::DecisionReapplied { succeeded: false })
                    }
                }
            }
        }
    }

    /// Seed the in-memory seq counter past the events already imported
    /// for this run, so a re-drive does not collide with — and get
    /// silently ignored against — historical seqs.
    async fn reseed_seq(
        &self,
        run_id: lazyboy_types::Id<lazyboy_types::domain::AgentRun>,
    ) -> Result<(), CoreError> {
        let count = repo::run::event_count(&self.store, run_id).await?;
        self.set_seq(run_id, count);
        Ok(())
    }
}
