use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use lazyboy_server::{router, AppState};
use lazyboy_store::{repo, Store};

/// GET /spaces returns 200 and the bootstrapped space as JSON, proving
/// the router serves the RpcClient read surface against a real store.
#[tokio::test]
async fn list_spaces_returns_bootstrapped_space() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let workspace = repo::bootstrap::create_workspace(&store, "default")
        .await
        .unwrap();
    repo::bootstrap::create_space(&store, workspace, "home", "Home")
        .await
        .unwrap();

    // No token: auth disabled (dev path), so no Authorization header.
    let app = router(AppState::new(
        store,
        "http://127.0.0.1:3284".to_owned(),
        None,
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/spaces")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let spaces: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(spaces.as_array().unwrap().len(), 1);
    assert_eq!(spaces[0]["slug"], "home");
    assert_eq!(spaces[0]["title"], "Home");
    assert_eq!(spaces[0]["status"], "active");
}

/// With a token configured, a request without the bearer is rejected.
#[tokio::test]
async fn missing_bearer_is_unauthorized() {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let app = router(AppState::new(
        store,
        "http://127.0.0.1:3284".to_owned(),
        Some("secret".to_owned()),
    ));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/spaces")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
