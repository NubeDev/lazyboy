//! Identities: timeline authors. Minted by `bootstrap::create_identity`;
//! this module is the read side used to resolve a principal by `kind`.

mod find_by_kind;

pub use find_by_kind::find_by_kind;
