use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use lazyboy_server::{router, AppState};
use lazyboy_store::{repo, Store};

/// The deterministic quick-add: `POST /spaces/:id/tasks` opens a task
/// with no agent run, and it then shows in the space's task list. This
/// is the `/task` command-bar path, proving a task can be created without
/// driving goose.
#[tokio::test]
async fn create_task_then_list() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let workspace = repo::bootstrap::create_workspace(&store, "default")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, workspace, "home", "Home")
        .await
        .unwrap();

    let app = router(AppState::new(store, "http://127.0.0.1:3284".to_owned(), None));

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/spaces/{space}/tasks"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"Ship the pricing page"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created_json: serde_json::Value =
        serde_json::from_slice(&created.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(created_json["title"], "Ship the pricing page");
    assert_eq!(created_json["state"], "open");
    assert!(created_json["agent_run_id"].is_null(), "quick-add has no run");

    let listed = app
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/tasks"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let tasks: serde_json::Value =
        serde_json::from_slice(&listed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["title"], "Ship the pricing page");
}

/// An empty title is a 400, not a blank task.
#[tokio::test]
async fn create_task_rejects_blank_title() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let workspace = repo::bootstrap::create_workspace(&store, "default")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, workspace, "home", "Home")
        .await
        .unwrap();

    let app = router(AppState::new(store, "http://127.0.0.1:3284".to_owned(), None));
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/spaces/{space}/tasks"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"   "}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
