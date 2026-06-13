//! Exercises the ingress dedup invariant (SCOPE.md "Integrations"):
//! ingesting the same `(integration_id, external_id)` twice yields one
//! timeline message; the second call returns that same message id and
//! writes no second ingress row.

use lazyboy_store::{repo, Store};
use lazyboy_types::domain::Provider;

#[tokio::test]
async fn ingest_dedups_on_external_id() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let ws = repo::bootstrap::create_workspace(&store, "acme")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, ws, "pricing", "Pricing")
        .await
        .unwrap();
    let author = repo::bootstrap::create_identity(
        &store,
        ws,
        repo::bootstrap::NewIdentity {
            kind: "agent",
            display_name: "Goose",
            external_ref: None,
        },
    )
    .await
    .unwrap();
    let integration = repo::integration::create(
        &store,
        repo::integration::NewIntegration {
            workspace_id: ws,
            provider: Provider::Github,
            account_ref: Some("acme"),
            secret_ref: Some("host://secrets/github"),
            config_json: None,
        },
    )
    .await
    .unwrap();

    let new = || repo::ingress::NewIngress {
        integration_id: integration,
        space_id: space,
        author,
        external_id: "comment:42",
        kind: "comment",
        payload_json: r#"{"comment":{"id":42}}"#,
        body: "[acme/web] comment: ship it",
    };

    let first = repo::ingress::ingest(&store, new()).await.unwrap();
    assert!(!first.deduped);

    let second = repo::ingress::ingest(&store, new()).await.unwrap();
    assert!(second.deduped, "a redelivery must be flagged as deduped");
    assert_eq!(
        first.message_id, second.message_id,
        "redelivery must resolve to the same message"
    );

    // Exactly one ingress row and one timeline message survive the
    // redelivery — the dedup invariant.
    let events = repo::ingress::list(&store, space).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].message_id, Some(first.message_id));

    let messages = repo::message::list(&store, space).await.unwrap();
    assert_eq!(messages.len(), 1);
}
