use sqlx::sqlite::SqliteRow;
use sqlx::Row;

use super::decode;
use crate::StoreError;
use lazyboy_types::domain::{ApprovalPolicy, TriggerKind, Workflow, WorkflowStatus, Workspace};
use lazyboy_types::Id;

/// A decoded `workflows` row — a saved, triggerable agent run (SCOPE.md
/// "Workflows and automation"). `status == enabled` is what SCOPE.md
/// names an automation; `approval_policy` is the per-workflow R6 gate.
#[derive(Debug, Clone)]
pub struct WorkflowRow {
    pub id: Id<Workflow>,
    pub workspace_id: Id<Workspace>,
    pub name: String,
    pub trigger_kind: TriggerKind,
    pub trigger_config_json: Option<String>,
    pub approval_policy: ApprovalPolicy,
    pub steps_json: String,
    pub status: WorkflowStatus,
}

impl WorkflowRow {
    pub(crate) fn from_row(row: &SqliteRow) -> Result<Self, StoreError> {
        Ok(Self {
            id: decode::id(row.try_get("id")?, "workflows.id")?,
            workspace_id: decode::id(row.try_get("workspace_id")?, "workflows.workspace_id")?,
            name: row.try_get("name")?,
            trigger_kind: decode::parse(row.try_get("trigger_kind")?, "workflows.trigger_kind")?,
            trigger_config_json: row.try_get("trigger_config_json")?,
            approval_policy: decode::parse(
                row.try_get("approval_policy")?,
                "workflows.approval_policy",
            )?,
            steps_json: row.try_get("steps_json")?,
            status: decode::parse(row.try_get("status")?, "workflows.status")?,
        })
    }
}
