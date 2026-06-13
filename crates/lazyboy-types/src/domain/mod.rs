//! Domain vocabulary. Aggregate markers (for `Id<T>`) and the state
//! enums from the SCOPE.md SQLite model, one concept per file.

mod approval_status;
mod marker;
mod message_kind;
mod run_status;
mod task_state;

pub use approval_status::ApprovalStatus;
pub use marker::{AgentRun, Approval, Identity, Message, Space, Task, Workspace};
pub use message_kind::MessageKind;
pub use run_status::RunStatus;
pub use task_state::TaskState;
