//! Ingress: external events become timeline messages in a bound space,
//! deduped through `ingress_events` (SCOPE.md "Integrations"). `ingest`
//! is the idempotent sink; `list` reads a space's ingress audit trail.

mod ingest;
mod list;

pub use ingest::{ingest, IngestOutcome, NewIngress};
pub use list::list;
