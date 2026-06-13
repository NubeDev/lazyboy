//! One handler per file, each mapping a single `RpcClient` method to its
//! HTTP route. Reads borrow only the store; the two mutating handlers
//! (start_run, decide) build an engine with the live goose transport.

mod create_integration;
mod create_reminder;
mod decide;
mod dismiss_reminder;
mod ingress;
mod list_calendar;
mod list_decisions;
mod list_integrations;
mod list_pending;
mod list_reminders;
mod list_runs;
mod list_spaces;
mod list_tasks;
mod record_decision;
mod start_run;
mod subscribe;
mod timeline;
mod upsert_calendar;

pub use create_integration::create_integration;
pub use create_reminder::create_reminder;
pub use decide::decide;
pub use dismiss_reminder::dismiss_reminder;
pub use ingress::ingress;
pub use list_calendar::list_calendar;
pub use list_decisions::list_decisions;
pub use list_integrations::list_integrations;
pub use list_pending::list_pending;
pub use list_reminders::list_reminders;
pub use list_runs::list_runs;
pub use list_spaces::list_spaces;
pub use list_tasks::list_tasks;
pub use record_decision::record_decision;
pub use start_run::start_run;
pub use subscribe::subscribe;
pub use timeline::timeline;
pub use upsert_calendar::upsert_calendar;
