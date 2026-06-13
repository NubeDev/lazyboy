use lazyboy_store::{repo, Store};
use lazyboy_types::domain::Space;
use lazyboy_types::Id;
use lazyboy_wire::MessageDto;

use crate::error::RpcError;

/// The number of timeline rows a subscriber has already seen. Carried
/// across polls so each tick emits only newly-appended messages, the
/// same high-water-mark bridge the HTTP shell's SSE loop uses (`emitted`
/// in `server::routes::subscribe`). The timeline is append-only (R1), so
/// a count is a sufficient cursor — no ordering key to track.
pub type Cursor = usize;

/// Read the space timeline and split off the rows appended since the
/// cursor, returning the projected DTOs and the advanced cursor. Pure
/// over the store so it is testable without any tauri emit; the desktop
/// poller (`app.rs`) calls this and emits each DTO over the space's Tauri
/// event channel, mirroring the server's per-message SSE `data:` events.
pub async fn new_messages_since(
    store: &Store,
    space_id: Id<Space>,
    cursor: Cursor,
) -> Result<(Vec<MessageDto>, Cursor), RpcError> {
    let rows = repo::message::list(store, space_id).await?;
    let total = rows.len();
    let fresh = rows
        .into_iter()
        .skip(cursor)
        .map(MessageDto::from)
        .collect();
    Ok((fresh, total))
}
