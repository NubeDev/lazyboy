use serde::{Deserialize, Serialize};

use lazyboy_core::RunOutcome;
use lazyboy_store::{
    ApprovalRow, CalendarEventRow, DecisionRow, IntegrationRow, MessageRow, ReminderRow, RunRow,
    SpaceRow, TaskRow,
};
use lazyboy_types::domain::{
    AgentRun, Approval, ApprovalStatus, CalendarEvent, Decision, Identity, Integration, Message,
    MessageKind, Provider, Reminder, ReminderStatus, RunStatus, Space, Task, TaskState, Workspace,
};
use lazyboy_types::Id;

/// RFC3339 rendering shared by the durable-memory DTOs, matching the
/// text columns and the JS `Date` parser; a formatting miss is reported
/// as an empty string rather than failing the whole response.
fn rfc3339(ts: time::OffsetDateTime) -> String {
    ts.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

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

#[derive(Serialize)]
pub struct DecisionDto {
    pub id: Id<Decision>,
    pub space_id: Id<Space>,
    pub message_id: Option<Id<Message>>,
    pub summary: String,
    pub decided_by_identity_id: Option<Id<Identity>>,
    pub decided_at: String,
}

impl From<DecisionRow> for DecisionDto {
    fn from(r: DecisionRow) -> Self {
        Self {
            id: r.id,
            space_id: r.space_id,
            message_id: r.message_id,
            summary: r.summary,
            decided_by_identity_id: r.decided_by_identity_id,
            decided_at: rfc3339(r.decided_at),
        }
    }
}

#[derive(Serialize)]
pub struct ReminderDto {
    pub id: Id<Reminder>,
    pub space_id: Id<Space>,
    pub task_id: Option<Id<Task>>,
    pub due_at: String,
    pub body: String,
    pub status: ReminderStatus,
}

impl From<ReminderRow> for ReminderDto {
    fn from(r: ReminderRow) -> Self {
        Self {
            id: r.id,
            space_id: r.space_id,
            task_id: r.task_id,
            due_at: rfc3339(r.due_at),
            body: r.body,
            status: r.status,
        }
    }
}

#[derive(Serialize)]
pub struct CalendarEventDto {
    pub id: Id<CalendarEvent>,
    pub space_id: Id<Space>,
    pub source: String,
    pub external_ref: Option<String>,
    pub title: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub meta_json: Option<String>,
}

impl From<CalendarEventRow> for CalendarEventDto {
    fn from(r: CalendarEventRow) -> Self {
        Self {
            id: r.id,
            space_id: r.space_id,
            source: r.source,
            external_ref: r.external_ref,
            title: r.title,
            starts_at: rfc3339(r.starts_at),
            ends_at: r.ends_at.map(rfc3339),
            meta_json: r.meta_json,
        }
    }
}

/// `POST /spaces/:id/decisions` body. `message_id` and the author are
/// optional, mirroring the row: a decision can be recorded without an
/// anchoring message.
#[derive(Deserialize)]
pub struct RecordDecisionBody {
    pub summary: String,
    #[serde(default)]
    pub message_id: Option<Id<Message>>,
    #[serde(default)]
    pub decided_by_identity_id: Option<Id<Identity>>,
}

/// `POST /spaces/:id/reminders` body. Timestamps cross the wire as
/// RFC3339 strings (the column format, shared with the TS client) and
/// are parsed in the handler; the `time` serde default is not RFC3339.
#[derive(Deserialize)]
pub struct CreateReminderBody {
    pub body: String,
    pub due_at: String,
    #[serde(default)]
    pub task_id: Option<Id<Task>>,
}

/// `POST /spaces/:id/calendar` body. `external_ref` present marks a
/// synced event the upsert dedups on; absent is a fresh local event.
/// Timestamps are RFC3339 strings parsed in the handler.
#[derive(Deserialize)]
pub struct UpsertCalendarBody {
    pub source: String,
    pub title: String,
    pub starts_at: String,
    #[serde(default)]
    pub external_ref: Option<String>,
    #[serde(default)]
    pub ends_at: Option<String>,
    #[serde(default)]
    pub meta_json: Option<String>,
}

#[derive(Deserialize)]
pub struct StartRunBody {
    pub prompt: String,
}

#[derive(Deserialize)]
pub struct DecisionBody {
    pub status: ApprovalStatus,
}

/// An integration projected for the UI. `secret_ref` is the only
/// credential surface and it names a host secrets-store entry, never the
/// raw token (SCOPE.md R5), so it is safe to project.
#[derive(Serialize)]
pub struct IntegrationDto {
    pub id: Id<Integration>,
    pub workspace_id: Id<Workspace>,
    pub provider: Provider,
    pub account_ref: Option<String>,
    pub secret_ref: Option<String>,
    pub status: String,
    pub config_json: Option<String>,
}

impl From<IntegrationRow> for IntegrationDto {
    fn from(r: IntegrationRow) -> Self {
        Self {
            id: r.id,
            workspace_id: r.workspace_id,
            provider: r.provider,
            account_ref: r.account_ref,
            secret_ref: r.secret_ref,
            status: r.status,
            config_json: r.config_json,
        }
    }
}

/// `POST /integrations` body. Carries `secret_ref` (a host secrets-store
/// pointer), never a raw secret (SCOPE.md R5). `config_json` holds the
/// explicit ingress bindings.
#[derive(Deserialize)]
pub struct CreateIntegrationBody {
    pub workspace_id: Id<Workspace>,
    pub provider: Provider,
    #[serde(default)]
    pub account_ref: Option<String>,
    #[serde(default)]
    pub secret_ref: Option<String>,
    #[serde(default)]
    pub config_json: Option<serde_json::Value>,
}

/// `POST /integrations/:id/ingress` body: a raw provider webhook/poll
/// `payload`, plus an optional explicit `space_id`. When `space_id` is
/// absent the bound space is resolved from the integration's
/// `config_json` bindings (SCOPE.md explicit-binding routing).
#[derive(Deserialize)]
pub struct IngressBody {
    pub payload: serde_json::Value,
    #[serde(default)]
    pub space_id: Option<Id<Space>>,
}

/// The result of an ingress POST: the timeline message the event mapped
/// to, and whether the call was a deduped redelivery (SCOPE.md ingress
/// idempotency boundary).
#[derive(Serialize)]
pub struct IngestResultDto {
    pub message_id: Id<Message>,
    pub deduped: bool,
}
