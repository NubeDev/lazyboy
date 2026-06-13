//! The trust layer (SCOPE.md R6). `request` writes the durable
//! pending row the instant Goose asks for a tool; `resolve` records a
//! human decision; `list_pending` feeds the approval queue;
//! `needs_resume` is the crash-reconcile query.

mod get;
mod list_pending;
mod needs_resume;
mod request;
mod resolve;

pub use get::get;
pub use list_pending::list_pending;
pub use needs_resume::needs_resume;
pub use request::{request, NewApproval};
pub use resolve::resolve;
