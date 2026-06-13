use super::ToolCall;

/// An agent->client `session/request_permission`. The bridge turns
/// this into a durable `approvals` row before answering; the answer is
/// sent only once a human resolves it in the timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermissionRequest {
    /// The ACP request id, echoed back with the `Decision` so goose
    /// correlates the answer to the in-flight tool call.
    pub request_id: String,
    pub tool: ToolCall,
}
