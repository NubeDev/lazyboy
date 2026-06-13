use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{AgentRun, Artifact, Space};
use lazyboy_types::Id;

/// A produced artifact imported from a tool result: a file written, a
/// URL reached, a patch, a PR. `uri` is what locates it; `meta_json`
/// carries the raw tool output it was distilled from, so the timeline
/// keeps the provenance even when the heuristic guesses the kind.
pub struct NewArtifact<'a> {
    pub space_id: Id<Space>,
    pub agent_run_id: Id<AgentRun>,
    pub kind: &'a str,
    pub uri: &'a str,
    pub meta_json: Option<&'a str>,
}

pub async fn create(store: &Store, new: NewArtifact<'_>) -> Result<Id<Artifact>, StoreError> {
    let id = Id::<Artifact>::new();
    sqlx::query(
        "INSERT INTO artifacts (id, space_id, agent_run_id, kind, uri, meta_json, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(new.space_id.to_string())
    .bind(new.agent_run_id.to_string())
    .bind(new.kind)
    .bind(new.uri)
    .bind(new.meta_json)
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(id)
}
