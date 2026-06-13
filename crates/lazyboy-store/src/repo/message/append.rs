use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{Identity, Message, MessageKind, Space};
use lazyboy_types::Id;

/// A message to append to a space timeline. `ref_id` points at the
/// approvals/artifacts/decisions/ingress row this message stands for
/// when its kind is one of the typed reference kinds.
pub struct NewMessage<'a> {
    pub space_id: Id<Space>,
    pub author: Id<Identity>,
    pub kind: MessageKind,
    pub body: &'a str,
    pub ref_id: Option<String>,
}

pub async fn append(store: &Store, msg: NewMessage<'_>) -> Result<Id<Message>, StoreError> {
    let id = Id::<Message>::new();
    sqlx::query(
        "INSERT INTO messages (id, space_id, author_identity_id, kind, body, ts, ref_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(msg.space_id.to_string())
    .bind(msg.author.to_string())
    .bind(msg.kind.as_str())
    .bind(msg.body)
    .bind(clock::fmt(clock::now()))
    .bind(msg.ref_id)
    .execute(store.pool())
    .await?;
    Ok(id)
}
