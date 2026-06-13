//! Workflow and membership store verbs (SCOPE.md "Workflows and
//! automation", "Feeds, membership, and visibility").

use lazyboy_store::{repo, Store};
use lazyboy_types::domain::{ApprovalPolicy, TriggerKind, WorkflowStatus};

async fn store() -> Store {
    Store::connect("sqlite::memory:").await.unwrap()
}

#[tokio::test]
async fn workflow_create_list_enable_disable() {
    let store = store().await;
    let ws = repo::bootstrap::create_workspace(&store, "acme")
        .await
        .unwrap();

    let id = repo::workflow::create(
        &store,
        repo::workflow::NewWorkflow {
            workspace_id: ws,
            name: "triage PRs",
            trigger_kind: TriggerKind::Feed,
            trigger_config_json: Some(r#"{"repo":"acme/web"}"#),
            approval_policy: ApprovalPolicy::RequireApproval,
            steps_json: r#"{"prompt":"triage"}"#,
        },
    )
    .await
    .unwrap();

    let created = repo::workflow::get(&store, id).await.unwrap();
    assert_eq!(
        created.status,
        WorkflowStatus::Disabled,
        "a workflow is saved inert until armed"
    );
    assert_eq!(created.approval_policy, ApprovalPolicy::RequireApproval);
    assert_eq!(created.trigger_kind, TriggerKind::Feed);

    let listed = repo::workflow::list(&store, ws).await.unwrap();
    assert_eq!(listed.len(), 1);

    repo::workflow::set_status(&store, id, WorkflowStatus::Enabled)
        .await
        .unwrap();
    let armed = repo::workflow::get(&store, id).await.unwrap();
    assert!(
        armed.status.is_automation(),
        "an enabled workflow is an automation"
    );

    repo::workflow::set_status(&store, id, WorkflowStatus::Disabled)
        .await
        .unwrap();
    assert_eq!(
        repo::workflow::get(&store, id).await.unwrap().status,
        WorkflowStatus::Disabled
    );
}

#[tokio::test]
async fn membership_create_group_add_member_grant() {
    let store = store().await;
    let ws = repo::bootstrap::create_workspace(&store, "acme")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, ws, "pricing", "Pricing")
        .await
        .unwrap();
    let ada = repo::bootstrap::create_identity(
        &store,
        ws,
        repo::bootstrap::NewIdentity {
            kind: "human",
            display_name: "Ada",
            external_ref: None,
        },
    )
    .await
    .unwrap();

    let group = repo::membership::create_group(&store, ws, "reviewers")
        .await
        .unwrap();
    repo::membership::add_member(&store, group, ada)
        .await
        .unwrap();
    // Idempotent: the composite PK makes a repeat add a no-op.
    repo::membership::add_member(&store, group, ada)
        .await
        .unwrap();

    let members = repo::membership::list_members(&store, group).await.unwrap();
    assert_eq!(members, vec![ada]);

    repo::membership::grant_membership(&store, space, "group", &group.to_string(), "editor")
        .await
        .unwrap();
    let memberships = repo::membership::list_memberships(&store, space)
        .await
        .unwrap();
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].principal_kind, "group");
    assert_eq!(memberships[0].role, "editor");
}
