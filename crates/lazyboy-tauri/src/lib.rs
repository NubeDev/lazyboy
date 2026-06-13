//! The desktop-shell backend: the lazyboy core exposed as Tauri 2
//! commands so the one React UI reaches it in-process through its
//! `TauriRpcClient` (SCOPE.md "UI: one React app, two shells"). The
//! second backend shell after `lazyboy-server`; both expose the same
//! `RpcClient` surface and reuse the same `lazyboy-wire` DTOs, so the
//! JSON the webview receives is identical across transports.
//!
//! The command bodies (`TauriRpc`) and the subscribe cursor logic are
//! plain async functions over the engine/store, built and tested on
//! default features. The `#[tauri::command]` wrappers and the
//! `Builder`/event-channel wiring live behind the `app` feature
//! (`app.rs`) so the GUI/webview system stack is only required for a real
//! desktop build.

mod error;
mod handle;
mod subscribe;

#[cfg(feature = "app")]
mod app;

pub use error::RpcError;
pub use handle::TauriRpc;
pub use subscribe::{new_messages_since, Cursor};

#[cfg(feature = "app")]
pub use app::run;
