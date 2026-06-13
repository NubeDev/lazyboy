use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{Decision, Identity, Message, Space};
use lazyboy_types::Id;

/// A decision to record into a space's durable memory. The `message_id`
/// anchors it to the point in the timeline where it was settled, so
/// "where was that decided?" is answerable; both it and the author are
/// optional because a decision can be synthesised without one.
pub struct NewDecision<'a> {
    pub space_id: Id<Space>,
    pub message_id: Option<Id<Message>>,
    pub summary: &'a str,
    pub decided_by_identity_id: Option<Id<Identity>>,
}

pub async fn record(store: &Store, new: NewDecision<'_>) -> Result<Id<Decision>, StoreError> {
    let id = Id::<Decision>::new();
    sqlx::query(
        "INSERT INTO decisions (id, space_id, message_id, summary, decided_by_identity_id, \
         decided_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(new.space_id.to_string())
    .bind(new.message_id.map(|m| m.to_string()))
    .bind(new.summary)
    .bind(new.decided_by_identity_id.map(|i| i.to_string()))
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(id)
}
