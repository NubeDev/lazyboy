//! Ingress normalization — provider payload in, normalized event out.
//!
//! The boundary between the world's many event shapes and the store's
//! single `ingress::ingest` verb (SCOPE.md). Each provider module is a
//! pure function: a `serde_json::Value` webhook/poll payload becomes a
//! `NormalizedEvent` (a stable `external_id`, a `kind`, human-readable
//! `body`). No transport lives here — fetching, OAuth, and polling are
//! out of MVP scope (DOCS/INGRESS.md) and would be host-side anyway, so
//! this crate stays mobile-safe (codeless R1) with zero process/socket
//! dependencies.
//!
//! Routing (which space an event lands in) is explicit binding, not
//! auto-routing (SCOPE.md): `binding` resolves an event to a space from
//! the bindings stored in `integrations.config_json`.

mod binding;
mod error;
mod event;
pub mod github;
pub mod gmail;

pub use binding::{resolve_space, Binding, Bindings};
pub use error::NormalizeError;
pub use event::NormalizedEvent;

use lazyboy_types::domain::Provider;
use serde_json::Value;

/// Normalize a raw provider payload by its declared provider. Slack and
/// Gcal are accepted providers (SCOPE.md priority order) but their
/// normalizers are not in this MVP slice; they report `Unsupported`.
pub fn normalize(provider: Provider, payload: &Value) -> Result<NormalizedEvent, NormalizeError> {
    match provider {
        Provider::Github => github::normalize(payload),
        Provider::Gmail => gmail::normalize(payload),
        Provider::Slack | Provider::Gcal => Err(NormalizeError::Unsupported(provider)),
    }
}
