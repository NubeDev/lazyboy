//! Repositories: one verb per file, grouped by aggregate folder
//! (FILE-LAYOUT.md). Each verb takes `&Store` and does exactly one
//! query. No repo struct, no trait — free functions keep the call
//! site honest about which table it touches.

mod clock;

pub mod approval;
pub mod artifact;
pub mod bootstrap;
pub mod calendar;
pub mod decision;
pub mod identity;
pub mod ingress;
pub mod integration;
pub mod membership;
pub mod message;
pub mod outbox;
pub mod reminder;
pub mod run;
pub mod space;
pub mod task;
pub mod workflow;
