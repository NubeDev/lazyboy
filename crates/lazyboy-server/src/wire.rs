use serde::{Deserialize, Serialize};

use lazyboy_core::RunOutcome;
use lazyboy_store::{ApprovalRow, MessageRow, RunRow, SpaceRow, TaskRow};
use lazyboy_types::domain::{
    AgentRun, Approval, ApprovalStatus, Message, MessageKind, RunStatus, Space, Task, TaskState,
    Workspace,
};
use lazyboy_types::Id;

/// Wire shapes for the `RpcClient` surface (SCOPE.md). The store row
/// types are not `Serialize` (they decode SQLite, a separate concern),
/// so these thin DTOs project them onto JSON. The enum fields stay the
/// domain types: their snake_case serde forms are the contract shared
/// verbatim with the TypeScript client, so we never restate them here.
#[derive(Serialize)]
pub struct SpaceDto {
    pub id: Id<Space>,
    pub workspace_id: Id<Workspace>,
    pub slug: String,
    pub title: String,
    pub status: String,
}

impl From<SpaceRow> for SpaceDto {
    fn from(r: SpaceRow) -> Self {
        Self {
            id: r.id,
            workspace_id: r.workspace_id,
            slug: r.slug,
            title: r.title,
            status: r.status,
        }
    }
}

#[derive(Serialize)]
pub struct MessageDto {
    pub id: Id<Message>,
    pub space_id: Id<Space>,
    pub kind: MessageKind,
    pub body: String,
    pub ts: String,
    pub ref_id: Option<String>,
}

impl From<MessageRow> for MessageDto {
    fn from(r: MessageRow) -> Self {
        Self {
            id: r.id,
            space_id: r.space_id,
            kind: r.kind,
            body: r.body,
            // RFC3339 matches the text column and the JS `Date` parser.
            ts: r
                .ts
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            ref_id: r.ref_id,
        }
    }
}

#[derive(Serialize)]
pub struct ApprovalDto {
    pub id: Id<Approval>,
    pub space_id: Id<Space>,
    pub agent_run_id: Id<AgentRun>,
    pub goose_session_id: String,
    pub tool_name: String,
    pub tool_input_json: String,
    pub status: ApprovalStatus,
}

impl From<ApprovalRow> for ApprovalDto {
    fn from(r: ApprovalRow) -> Self {
        Self {
            id: r.id,
            space_id: r.space_id,
            agent_run_id: r.agent_run_id,
            goose_session_id: r.goose_session_id,
            tool_name: r.tool_name,
            tool_input_json: r.tool_input_json,
            status: r.status,
        }
    }
}

#[derive(Serialize)]
pub struct TaskDto {
    pub id: Id<Task>,
    pub space_id: Id<Space>,
    pub title: String,
    pub state: TaskState,
    pub agent_run_id: Option<Id<AgentRun>>,
}

impl From<TaskRow> for TaskDto {
    fn from(r: TaskRow) -> Self {
        Self {
            id: r.id,
            space_id: r.space_id,
            title: r.title,
            state: r.state,
            agent_run_id: r.agent_run_id,
        }
    }
}

#[derive(Serialize)]
pub struct RunDto {
    pub id: Id<AgentRun>,
    pub space_id: Id<Space>,
    pub task_id: Id<Task>,
    pub goose_session_id: Option<String>,
    pub status: RunStatus,
}

impl From<RunRow> for RunDto {
    fn from(r: RunRow) -> Self {
        Self {
            id: r.id,
            space_id: r.space_id,
            task_id: r.task_id,
            goose_session_id: r.goose_session_id,
            status: r.status,
        }
    }
}

/// Where a run paused, mirroring `core::RunOutcome`. The `outcome` tag is
/// the snake_case discriminant the TS union keys off.
#[derive(Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RunOutcomeDto {
    AwaitingApproval,
    Ended {
        succeeded: bool,
    },
    /// `decide` returns no outcome when the approval was already resolved
    /// by a racing client (single-tenant, but two browser tabs can race).
    AlreadyResolved,
}

impl From<RunOutcome> for RunOutcomeDto {
    fn from(o: RunOutcome) -> Self {
        match o {
            RunOutcome::AwaitingApproval => Self::AwaitingApproval,
            RunOutcome::Ended { succeeded } => Self::Ended { succeeded },
        }
    }
}

#[derive(Deserialize)]
pub struct StartRunBody {
    pub prompt: String,
}

#[derive(Deserialize)]
pub struct DecisionBody {
    pub status: ApprovalStatus,
}
