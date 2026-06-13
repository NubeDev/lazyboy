/// A Goose session handle (`acp-session-id`). Opaque to Lazyboy; we
/// store it on `agent_runs.goose_session_id` and pass it back to
/// `session/load` on resume.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
