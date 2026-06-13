//! The append-only space timeline. Messages are never mutated; a
//! correction is a new message. `append` covers every kind (human,
//! agent, tool_request/result, ...); `list` reads one space in order.

mod append;
mod list;

pub use append::{append, NewMessage};
pub use list::list;
