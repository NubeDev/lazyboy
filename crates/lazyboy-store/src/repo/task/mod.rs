//! Tasks: a unit of work in a space, optionally bound to one agent
//! run. `create` opens it, `set_state` advances the lifecycle,
//! `attach_run` records the run driving it, `get` reads one back.

mod attach_run;
mod create;
mod get;
mod list;
mod set_state;

pub use attach_run::attach_run;
pub use create::create;
pub use get::get;
pub use list::list;
pub use set_state::set_state;
