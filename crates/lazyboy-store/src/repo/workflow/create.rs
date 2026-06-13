use crate::repo::clock;
use crate::{Store, StoreError};
use lazyboy_types::domain::{ApprovalPolicy, TriggerKind, Workflow, WorkflowStatus, Workspace};
use lazyboy_types::Id;

/// A workflow to save (SCOPE.md "Workflows and automation"). A workflow
/// is created `disabled`; enabling it via `set_status` arms its trigger
/// and makes it an automation. `trigger_config_json` is what the
/// workflow agent matches feed/ingress events against; `steps_json`
/// carries the prompt and any inter-step approval checkpoints.
pub struct NewWorkflow<'a> {
    pub workspace_id: Id<Workspace>,
    pub name: &'a str,
    pub trigger_kind: TriggerKind,
    pub trigger_config_json: Option<&'a str>,
    pub approval_policy: ApprovalPolicy,
    pub steps_json: &'a str,
}

pub async fn create(store: &Store, new: NewWorkflow<'_>) -> Result<Id<Workflow>, StoreError> {
    let id = Id::<Workflow>::new();
    sqlx::query(
        "INSERT INTO workflows (id, workspace_id, name, trigger_kind, trigger_config_json, \
         approval_policy, steps_json, status, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.to_string())
    .bind(new.workspace_id.to_string())
    .bind(new.name)
    .bind(new.trigger_kind.as_str())
    .bind(new.trigger_config_json)
    .bind(new.approval_policy.as_str())
    .bind(new.steps_json)
    .bind(WorkflowStatus::Disabled.as_str())
    .bind(clock::fmt(clock::now()))
    .execute(store.pool())
    .await?;
    Ok(id)
}
