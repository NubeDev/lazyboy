use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::stream::Stream;

use lazyboy_store::repo;
use lazyboy_types::domain::Space;
use lazyboy_types::Id;

use crate::state::AppState;
use crate::wire::MessageDto;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// `GET /spaces/:id/subscribe` -> SSE stream of appended timeline
/// messages (RpcClient.subscribe).
///
/// This polls the store timeline and emits each newly-appended message
/// as a `data:` event. It is a deliberate bridge: the store has no
/// broadcast channel yet, and a half-second poll over an append-only
/// table delivers the same "UI subscribes to events" contract the Tauri
/// shell gets over its event channel. Replace the poll loop with a
/// store-side subscription when one lands; the wire shape stays the same.
pub async fn subscribe(
    State(state): State<AppState>,
    Path(space_id): Path<Id<Space>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let mut emitted = 0usize;
        loop {
            if let Ok(rows) = repo::message::list(state.store(), space_id).await {
                for row in rows.into_iter().skip(emitted) {
                    emitted += 1;
                    let dto = MessageDto::from(row);
                    match Event::default().json_data(&dto) {
                        Ok(event) => yield Ok(event),
                        Err(_) => continue,
                    }
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}
