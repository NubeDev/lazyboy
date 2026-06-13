/// A tool Goose wants to run, as named in a `session/request_permission`.
/// `input` is the raw JSON arguments, stored verbatim on the approval
/// row so the human sees exactly what would execute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub name: String,
    pub input_json: String,
}
