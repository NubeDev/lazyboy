//! Decoded row types: the in-memory shape of a timeline table after
//! its text columns are parsed into typed enums and ids. One row type
//! per file; each owns its `FromRow`-style decode from a sqlx row.

mod approval;
mod decode;
mod message;
mod run;
mod space;
mod task;

pub use approval::ApprovalRow;
pub use message::MessageRow;
pub use run::RunRow;
pub use space::SpaceRow;
pub use task::TaskRow;
