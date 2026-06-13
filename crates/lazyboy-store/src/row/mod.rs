//! Decoded row types: the in-memory shape of a timeline table after
//! its text columns are parsed into typed enums and ids. One row type
//! per file; each owns its `FromRow`-style decode from a sqlx row.

mod approval;
mod artifact;
mod calendar_event;
mod decision;
mod decode;
mod feed_visibility;
mod group;
mod ingress_event;
mod integration;
mod message;
mod outbox_event;
mod reminder;
mod run;
mod space;
mod space_membership;
mod task;
mod workflow;
mod workflow_run;

pub use approval::ApprovalRow;
pub use artifact::ArtifactRow;
pub use calendar_event::CalendarEventRow;
pub use decision::DecisionRow;
pub use feed_visibility::FeedVisibilityRow;
pub use group::GroupRow;
pub use ingress_event::IngressEventRow;
pub use integration::IntegrationRow;
pub use message::MessageRow;
pub use outbox_event::OutboxEventRow;
pub use reminder::ReminderRow;
pub use run::RunRow;
pub use space::SpaceRow;
pub use space_membership::SpaceMembershipRow;
pub use task::TaskRow;
pub use workflow::WorkflowRow;
pub use workflow_run::WorkflowRunRow;
