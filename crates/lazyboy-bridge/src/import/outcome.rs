use lazyboy_types::domain::Approval;
use lazyboy_types::Id;

/// What importing one update did, so the driver can react: a captured
/// approval pauses the run until resolved; a turn end closes it.
#[derive(Debug, PartialEq, Eq)]
pub enum Imported {
    /// An ordinary update (a tool result) landed in the timeline; the
    /// driver keeps pulling.
    Recorded,

    /// A streamed agent-message chunk. Its event row is already recorded
    /// (the audit log is one row per chunk), but the timeline text is
    /// returned to the driver to coalesce: goose streams a turn token by
    /// token, and one timeline message per token is unreadable. The
    /// driver buffers consecutive chunks and appends a single agent
    /// message when the run reaches any other update or the turn ends.
    /// Coalescing in the driver (not by mutating a row) keeps messages
    /// append-only, which the outbox union-merge sync relies on.
    AgentChunk { text: String },

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
