//! Zenoh sync fabric (SCOPE.md "Zenoh sync fabric", build step 5).
//!
//! The team layer rides on the store's `outbox_events`, not on bespoke
//! replication: a publisher drains the outbox and ships each event, a
//! subscriber applies inbound events back into the local store. SQLite
//! stays the source of truth (R1); Zenoh only moves events between nodes.
//!
//! All networking is feature-gated behind `zenoh` (default off) so the
//! workspace builds and tests with no network and no heavy transport
//! dependency. The load-bearing, always-compiled logic is pure and
//! tested: the LWW merge decision ([`merge`], [`apply::decide`]), the
//! outbox-to-wire mapping ([`drain`]), and the event serde ([`event`]).
//! See DOCS/ZENOH.md for the merge-rule resolution and the integration
//! checklist.

pub mod apply;
pub mod config;
pub mod drain;
pub mod event;
pub mod merge;

mod error;
pub use error::SyncError;

#[cfg(feature = "zenoh")]
mod inbound;
#[cfg(feature = "zenoh")]
pub mod session;
