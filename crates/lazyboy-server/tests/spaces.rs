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

/// The SSE subscribe endpoint authorises via a `token` query param,
/// because `EventSource` cannot set an Authorization header. A correct
/// query token passes the gate (the request reaches the handler, which
/// then streams), while a wrong one is rejected.
#[tokio::test]
async fn subscribe_accepts_token_query_param() {
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
        Some("secret".to_owned()),
    ));

    let bad = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/subscribe?token=wrong"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);

    // The stream never ends, so assert the gate passed (status OK +
    // SSE content type) rather than draining the body.
    let ok = app
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/subscribe?token=secret"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
    assert!(ok
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));
}
