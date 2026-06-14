//! The bridge: drives Goose over the ACP seam and imports its events
//! into the timeline store. See `DOCS/GOOSE-ACP.md` for the verified
//! goose-1.37.0 contract this models.
//!
//! Goose is reached only through `GooseClient` (SCOPE.md R3). The live
//! HTTP+WebSocket transport is a host concern injected by the shell;
//! tests and `lazyboy-core` use `FakeGoose`. Everything here is
//! transport-agnostic.

mod acp;
mod error;
mod import;

pub use acp::{Decision, FakeGoose, GooseClient, PermissionRequest, SessionId, ToolCall, Update};
pub use error::BridgeError;
pub use import::{append_agent_message, import_update, ImportContext, Imported};
