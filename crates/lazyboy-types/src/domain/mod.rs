//! Domain vocabulary. Aggregate markers (for `Id<T>`) and the state
//! enums from the SCOPE.md SQLite model, one concept per file.

mod approval_policy;
mod approval_status;
mod marker;
mod message_kind;
mod provider;
mod reminder_status;
mod run_status;
mod task_state;
mod trigger_kind;
mod workflow_status;

pub use approval_policy::ApprovalPolicy;
pub use approval_status::ApprovalStatus;
pub use marker::{
    AgentRun, Approval, Artifact, CalendarEvent, Decision, Group, Identity, IngressEvent,
    Integration, Message, OutboxEvent, Reminder, Space, Task, Workflow, WorkflowRun, Workspace,
};
pub use message_kind::MessageKind;
pub use provider::Provider;
pub use reminder_status::ReminderStatus;
pub use run_status::RunStatus;
pub use task_state::TaskState;
pub use trigger_kind::TriggerKind;
pub use workflow_status::WorkflowStatus;
