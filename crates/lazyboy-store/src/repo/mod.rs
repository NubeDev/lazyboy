//! Repositories: one verb per file, grouped by aggregate folder
//! (FILE-LAYOUT.md). Each verb takes `&Store` and does exactly one
//! query. No repo struct, no trait — free functions keep the call
//! site honest about which table it touches.

mod clock;

pub mod approval;
pub mod bootstrap;
pub mod message;
pub mod run;
pub mod task;
