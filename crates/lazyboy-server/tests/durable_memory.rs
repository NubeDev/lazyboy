use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use lazyboy_server::{router, AppState};
use lazyboy_store::{repo, Store};

/// POSTing a decision then GETting the space's decisions round-trips
/// through the router and the store, proving the new durable-memory
/// write+read surface is wired.
#[tokio::test]
async fn record_then_list_decisions() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let workspace = repo::bootstrap::create_workspace(&store, "default")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, workspace, "home", "Home")
        .await
        .unwrap();

    let app = router(AppState::new(
        store,
        "http://127.0.0.1:3284".to_owned(),
        None,
    ));

    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/spaces/{space}/decisions"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"summary":"go with sqlite"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);

    let listed = app
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/decisions"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);

    let bytes = listed.into_body().collect().await.unwrap().to_bytes();
    let decisions: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decisions.as_array().unwrap().len(), 1);
    assert_eq!(decisions[0]["summary"], "go with sqlite");
}
