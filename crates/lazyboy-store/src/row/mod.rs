//! Decoded row types: the in-memory shape of a timeline table after
//! its text columns are parsed into typed enums and ids. One row type
//! per file; each owns its `FromRow`-style decode from a sqlx row.

mod approval;
mod artifact;
mod calendar_event;
mod decision;
mod decode;
mod ingress_event;
mod integration;
mod message;
mod outbox_event;
mod reminder;
mod run;
mod space;
mod task;

pub use approval::ApprovalRow;
pub use artifact::ArtifactRow;
pub use calendar_event::CalendarEventRow;
pub use decision::DecisionRow;
pub use ingress_event::IngressEventRow;
pub use integration::IntegrationRow;
pub use message::MessageRow;
pub use outbox_event::OutboxEventRow;
pub use reminder::ReminderRow;
pub use run::RunRow;
pub use space::SpaceRow;
pub use task::TaskRow;
