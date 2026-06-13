use super::PermissionRequest;

/// One item off a session's update stream. Goose's `session/update`
/// notifications and `session/request_permission` requests are
/// normalised into this enum; the bridge's import step maps each to a
/// store write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Update {
    /// Assistant text/content the agent produced. Imported as an
    /// `agent` timeline message and a run event.
    AgentMessage { text: String },

    /// A tool ran and returned (no approval needed, or post-approval).
    /// Imported as a `tool_result` message and a run event.
    ToolResult {
        tool_name: String,
        output_json: String,
    },

    /// Goose is asking to run a gated tool. The bridge captures this
    /// as a pending `approvals` row and blocks until resolved.
    PermissionRequested(PermissionRequest),

    /// The agent turn finished. `stopped` is true on a clean end of
    /// turn, false if goose reported the run failed.
    TurnEnded { stopped: bool },
}
