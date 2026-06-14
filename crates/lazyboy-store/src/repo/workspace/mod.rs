//! Workspaces: the node hosts exactly one (single trust boundary,
//! SCOPE R5). This module is the read side; the row is minted by
//! `bootstrap::create_workspace`.

mod current;

pub use current::current;
