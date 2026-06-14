//! The lazyboy MCP endpoint (`/mcp`) is what lets the rented agent act on
//! a space. These drive it as goose would — JSON-RPC over POST — without a
//! live goose, so the keystone has CI coverage independent of a model
//! provider.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

use lazyboy_server::{router, AppState};
use lazyboy_store::{repo, Store};

async fn rpc(app: &axum::Router, space: Option<&str>, body: &str) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json");
    if let Some(s) = space {
        req = req.header("X-Lazyboy-Space", s);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, json)
}

async fn app_with_space() -> (axum::Router, String) {
    let store = Store::connect("sqlite::memory:").await.unwrap();
    let workspace = repo::bootstrap::create_workspace(&store, "default")
        .await
        .unwrap();
    let space = repo::bootstrap::create_space(&store, workspace, "home", "Home")
        .await
        .unwrap();
    (
        router(AppState::new(store, "http://127.0.0.1:3284".to_owned(), None)),
        space.to_string(),
    )
}

#[tokio::test]
async fn initialize_and_list_tools() {
    let (app, space) = app_with_space().await;

    let (st, init) = rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(init["result"]["serverInfo"]["name"], "lazyboy");
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");

    let (_, tools) = rpc(&app, Some(&space), r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).await;
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"space_overview"));
    assert!(names.contains(&"list_tasks"));
    assert!(names.contains(&"create_task"));

    // Read tools must be hinted read-only so goose's gate lets them pass
    // without an approval prompt.
    let overview = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "space_overview")
        .unwrap();
    assert_eq!(overview["annotations"]["readOnlyHint"], true);
}

#[tokio::test]
async fn create_task_tool_lands_a_row() {
    let (app, space) = app_with_space().await;

    let (st, called) = rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"create_task","arguments":{"title":"Draft the RFC"}}}"#,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(called["result"]["isError"], false);

    // The task is now visible through the normal read path.
    let listed = app
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/tasks"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let tasks: Value =
        serde_json::from_slice(&listed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(tasks.as_array().unwrap().len(), 1);
    assert_eq!(tasks[0]["title"], "Draft the RFC");
}

#[tokio::test]
async fn set_task_state_closes_a_task() {
    let (app, space) = app_with_space().await;

    // Create, then read its id back through list_tasks (as the agent does).
    rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_task","arguments":{"title":"abc"}}}"#,
    )
    .await;
    let (_, listed) = rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"list_tasks","arguments":{}}}"#,
    )
    .await;
    let text = listed["result"]["content"][0]["text"].as_str().unwrap();
    let id = text
        .rsplit_once("id: ")
        .and_then(|(_, rest)| rest.split(')').next())
        .unwrap()
        .trim();

    let body = format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"set_task_state","arguments":{{"task_id":"{id}","state":"done"}}}}}}"#
    );
    let (st, res) = rpc(&app, Some(&space), &body).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(res["result"]["isError"], false);

    // The state change is visible through the normal read path.
    let listed = app
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/tasks"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let tasks: Value =
        serde_json::from_slice(&listed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(tasks[0]["state"], "done");
}

#[tokio::test]
async fn set_reminder_tool_lands_a_row() {
    let (app, space) = app_with_space().await;
    let (st, res) = rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"set_reminder","arguments":{"body":"Call Sam","due_at":"2026-06-14T13:00:00Z"}}}"#,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(res["result"]["isError"], false);

    let listed = app
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/reminders"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let reminders: Value =
        serde_json::from_slice(&listed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(reminders.as_array().unwrap().len(), 1);
    assert_eq!(reminders[0]["body"], "Call Sam");
}

#[tokio::test]
async fn set_reminder_rejects_bad_time() {
    let (app, space) = app_with_space().await;
    let (_, res) = rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"set_reminder","arguments":{"body":"x","due_at":"tomorrow"}}}"#,
    )
    .await;
    // A bad time is a tool error the agent can react to, not a crash.
    assert_eq!(res["result"]["isError"], true);
}

#[tokio::test]
async fn create_calendar_event_tool_lands_a_row() {
    let (app, space) = app_with_space().await;
    let (st, res) = rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"create_calendar_event","arguments":{"title":"Standup","starts_at":"2026-06-15T09:00:00Z"}}}"#,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(res["result"]["isError"], false);

    let listed = app
        .oneshot(
            Request::builder()
                .uri(format!("/spaces/{space}/calendar"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let events: Value =
        serde_json::from_slice(&listed.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(events.as_array().unwrap().len(), 1);
    assert_eq!(events[0]["title"], "Standup");
}

#[tokio::test]
async fn tool_call_without_space_header_is_an_error() {
    let (app, _space) = app_with_space().await;
    let (st, res) = rpc(
        &app,
        None,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"list_tasks","arguments":{}}}"#,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(res["error"]["code"], -32602);
}

#[tokio::test]
async fn notification_is_acknowledged_empty() {
    let (app, space) = app_with_space().await;
    let (st, body) = rpc(
        &app,
        Some(&space),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(body, Value::Null);
}
