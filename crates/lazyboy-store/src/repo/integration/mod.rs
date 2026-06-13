//! Integrations: the external feeds bound to the workspace (SCOPE.md
//! "Integrations"). Create stores only a `secret_ref` to the host
//! secrets store, never raw creds (R5); list and get project the rows.

mod create;
mod get;
mod list;

pub use create::{create, NewIntegration};
pub use get::get;
pub use list::list;
