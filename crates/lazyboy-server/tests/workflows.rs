//! The workflow HTTP surface (SCOPE.md "Workflows and automation"):
//! create, list, enable, and fire. Firing needs a live goose, which the
//! sandbox has none of, so the fire assertion confirms the route reaches
//! the engine and fails at the transport (502) rather than 404/400 — the
//! create/list/enable path is fully exercised against a real store.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use lazyboy_server::{router, AppState};
use lazyboy_store::{repo, Store};

#[tokio::test]
async fn create_list_enable_then_fire_workflow() {
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

    let app = router(AppState::new(
        store,
        "http://127.0.0.1:3284".to_owned(),
        None,
    ));

    // Create.
    let body = format!(
        r#"{{"workspace_id":"{ws}","name":"triage","trigger_kind":"feed","approval_policy":"auto_approve","steps_json":"do it"}}"#
    );
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/workflows")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created_json: serde_json::Value =
        serde_json::from_slice(&created.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(created_json["status"], "disabled");
    assert_eq!(created_json["approval_policy"], "auto_approve");
    let workflow_id = created_json["id"].as_str().unwrap().to_owned();

    // List.
    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/workflows?workspace_id={ws}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let listed_json: serde_json::Value =
        serde_json::from_slice(&listed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(listed_json.as_array().unwrap().len(), 1);

    // Enable: now an automation.
    let enabled = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/workflows/{workflow_id}/enable"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enabled.status(), StatusCode::OK);
    let enabled_json: serde_json::Value =
        serde_json::from_slice(&enabled.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(enabled_json["status"], "enabled");

    // Fire: the route reaches the engine and fails at the goose
    // transport (no live goosed in the sandbox), proving the endpoint is
    // wired through to run_workflow rather than 404/400.
    let fired = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/workflows/{workflow_id}/fire"))
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"space_id":"{space}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        fired.status(),
        StatusCode::BAD_GATEWAY,
        "fire reached the engine; only the goose transport was unavailable"
    );
}
