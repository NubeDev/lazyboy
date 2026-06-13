//! One handler per file, each mapping a single `RpcClient` method to its
//! HTTP route. Reads borrow only the store; the two mutating handlers
//! (start_run, decide) build an engine with the live goose transport.

mod decide;
mod list_pending;
mod list_runs;
mod list_spaces;
mod list_tasks;
mod start_run;
mod subscribe;
mod timeline;

pub use decide::decide;
pub use list_pending::list_pending;
pub use list_runs::list_runs;
pub use list_spaces::list_spaces;
pub use list_tasks::list_tasks;
pub use start_run::start_run;
pub use subscribe::subscribe;
pub use timeline::timeline;
