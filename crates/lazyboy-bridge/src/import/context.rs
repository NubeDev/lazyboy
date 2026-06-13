use lazyboy_types::domain::{AgentRun, Identity, Space};
use lazyboy_types::Id;

/// What an import needs to know to attribute writes: which run/space
/// the update belongs to, the Goose session backing it, and the agent
/// identity that authors imported timeline messages.
#[derive(Clone)]
pub struct ImportContext {
    pub space_id: Id<Space>,
    pub agent_run_id: Id<AgentRun>,
    pub goose_session_id: String,
    pub agent_identity: Id<Identity>,
}
