//! Agent runs and their imported event stream. A run is scoped to a
//! space+task and tracks the Goose session driving it. `append_event`
//! is the sink for SSE/WebSocket updates imported by the bridge.

mod append_event;
mod create;
mod event_count;
mod get;
mod list;
mod prompt_of;
mod set_session;
mod set_status;

pub use append_event::{append_event, NewRunEvent};
pub use create::create;
pub use event_count::event_count;
pub use get::get;
pub use list::list;
pub use prompt_of::prompt_of;
pub use set_session::set_session;
pub use set_status::set_status;
