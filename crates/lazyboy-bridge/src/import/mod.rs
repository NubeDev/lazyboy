//! Translate one ACP `Update` into timeline writes. The driver in
//! `lazyboy-core` owns the loop and the run lifecycle; this module owns
//! the single-event mapping so the two concerns stay separable and
//! independently testable.

mod context;
mod outcome;
mod update;

pub use context::ImportContext;
pub use outcome::Imported;
pub use update::import_update;
