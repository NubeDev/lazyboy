//! The ACP seam, modelled as Lazyboy needs it — not a full ACP
//! implementation. The bridge consumes a stream of `Update`s and
//! answers `PermissionRequest`s; that is the entire surface the
//! timeline depends on. The live transport maps goose's JSON-RPC onto
//! these types; `FakeGoose` produces them directly.

mod client;
mod decision;
mod fake;
mod permission;
mod session_id;
mod tool_call;
mod update;

pub use client::GooseClient;
pub use decision::Decision;
pub use fake::FakeGoose;
pub use permission::PermissionRequest;
pub use session_id::SessionId;
pub use tool_call::ToolCall;
pub use update::Update;
