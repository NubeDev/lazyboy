use lazyboy_types::domain::Approval;
use lazyboy_types::Id;

/// What importing one update did, so the driver can react: a captured
/// approval pauses the run until resolved; a turn end closes it.
#[derive(Debug, PartialEq, Eq)]
pub enum Imported {
    /// An ordinary update (agent text or tool result) landed in the
    /// timeline; the driver keeps pulling.
    Recorded,

    /// A permission request was captured as a pending approval. The
    /// driver must stop pulling and wait for a human, carrying the ACP
    /// `request_id` so it can answer once resolved.
    AwaitingApproval {
        approval_id: Id<Approval>,
        request_id: String,
    },

    /// The agent turn ended. `succeeded` distinguishes a clean stop
    /// from a reported failure.
    TurnEnded { succeeded: bool },
}
