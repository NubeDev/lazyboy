//! Node bootstrap: create the single workspace, a space, and the
//! identities that author timeline rows. MVP is one workspace, one
//! trust boundary (SCOPE.md R4); these verbs run once at setup.

mod create_identity;
mod create_space;
mod create_workspace;

pub use create_identity::{create_identity, NewIdentity};
pub use create_space::create_space;
pub use create_workspace::create_workspace;
