use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{AgentRun, Artifact, Space};
use lazyboy_types::Id;

/// A decoded `artifacts` row.
#[derive(Debug, Clone)]
pub struct ArtifactRow {
    pub id: Id<Artifact>,
    pub space_id: Id<Space>,
    pub agent_run_id: Option<Id<AgentRun>>,
    pub kind: String,
    pub uri: String,
    pub meta_json: Option<String>,
}

impl ArtifactRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        let agent_run_id = row
            .try_get::<Option<String>, _>("agent_run_id")?
            .map(|v| decode::id(&v, "artifacts.agent_run_id"))
            .transpose()?;
        Ok(Self {
            id: decode::id(row.try_get("id")?, "artifacts.id")?,
            space_id: decode::id(row.try_get("space_id")?, "artifacts.space_id")?,
            agent_run_id,
            kind: row.try_get("kind")?,
            uri: row.try_get("uri")?,
            meta_json: row.try_get("meta_json")?,
        })
    }
}
