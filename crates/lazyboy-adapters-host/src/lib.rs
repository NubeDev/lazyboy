//! Host-side live transport for the Goose seam. Implements the
//! `lazyboy_bridge::GooseClient` contract over `goose serve`'s
//! ACP-over-HTTP surface (one WebSocket + `POST /acp`), verified against
//! goose 1.37.0 in `DOCS/GOOSE-ACP.md`.
//!
//! This is the only crate that opens sockets to goose; keeping reqwest
//! and tungstenite here keeps `lazyboy-bridge` and the mobile-safe
//! crates transport-free. The engine is generic over `GooseClient`, so
//! the shell injects `GooseServeClient` in production and tests use
//! `FakeGoose` unchanged.

mod client;
mod conn;
mod wire;

pub use client::GooseServeClient;
