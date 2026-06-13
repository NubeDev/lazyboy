use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use lazyboy_server::{router, AppState};
use lazyboy_store::{repo, Store};
use lazyboy_types::domain::Provider;

/// POSTing a GitHub ingress event with an explicit space lands a message
/// and returns its id; a redelivery of the same payload is flagged
/// `deduped` and resolves to the same message (SCOPE.md ingress
/// idempotency boundary).
#[tokio::test]
async fn ingress_event_lands_message_and_dedups() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let ws = repo::bootstrap::create_workspace(&store, "default")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, ws, "home", "Home")
        .await
        .unwrap();
    repo::bootstrap::create_identity(
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
            account_ref: None,
            secret_ref: Some("host://secrets/github"),
            config_json: None,
        },
    )
    .await
    .unwrap();

    let app = router(AppState::new(
        store,
        "http://127.0.0.1:3284".to_owned(),
        None,
    ));

    let payload = format!(
        r#"{{"payload":{{"action":"created","repository":{{"full_name":"acme/web"}},"comment":{{"id":42,"body":"ship it"}}}},"space_id":"{space}"}}"#
    );

    let post = |body: String| {
        Request::builder()
            .method("POST")
            .uri(format!("/integrations/{integration}/ingress"))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    };

    let first = app.clone().oneshot(post(payload.clone())).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_json: serde_json::Value =
        serde_json::from_slice(&first.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(first_json["deduped"], false);
    let message_id = first_json["message_id"].as_str().unwrap().to_owned();

    let second = app.clone().oneshot(post(payload)).await.unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second_json: serde_json::Value =
        serde_json::from_slice(&second.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(second_json["deduped"], true);
    assert_eq!(second_json["message_id"].as_str().unwrap(), message_id);
}
