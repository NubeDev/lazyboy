//! Workflows: saved, triggerable agent runs (SCOPE.md "Workflows and
//! automation"). `create` saves a workflow disabled; `set_status`
//! arms/disarms it (enabled == automation); `record_run`/`finish_run`
//! link each firing to the agent run it created.

mod create;
mod get;
mod list;
mod list_runs;
mod record_run;
mod set_status;

pub use create::{create, NewWorkflow};
pub use get::get;
pub use list::list;
pub use list_runs::list_runs;
pub use record_run::{finish_run, record_run};
pub use set_status::set_status;
