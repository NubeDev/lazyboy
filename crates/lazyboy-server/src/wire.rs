//! The wire DTOs now live in `lazyboy-wire`, shared verbatim with the
//! Tauri desktop shell so both backends emit the same JSON to the one
//! React UI (SCOPE.md "UI: one React app, two shells"). Re-exported here
//! so the handlers keep referring to `crate::wire::*` unchanged.
pub use lazyboy_wire::*;
