//! The browser-shell backend: the lazyboy core exposed over HTTP+SSE so
//! the one React UI can reach it through its `HttpRpcClient` (SCOPE.md
//! "UI: one React app, two shells"). Routes map one-to-one onto the
//! `RpcClient` surface; the wire enums are the domain types' snake_case
//! serde forms, shared verbatim with the TypeScript client.
//!
//! This crate owns the live goose transport (through adapters-host) and
//! never reaches into goose itself, keeping the no-fork and crate
//! direction rules intact (SCOPE.md R3, codeless R1).

mod auth;
mod error;
mod mcp;
mod routes;
mod state;
mod wire;

use std::net::SocketAddr;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};

use lazyboy_store::Store;

pub use state::AppState;

/// Build the router for a given state. Split out from `serve` so tests
/// drive it with `tower::ServiceExt::oneshot` against an in-memory store.
pub fn router(state: AppState) -> Router {
    // CORS is first-class: the browser shell is a different origin than
    // this server (SCOPE.md). Permissive for the single-tenant bearer —
    // there is one trust boundary, so origin is not the gate, the token
    // is (R4). The bearer rides the Authorization header, which `Any`
    // headers admits.
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(routes::health))
        .route("/goose/providers", get(routes::list_goose_providers))
        .route(
            "/goose/config",
            get(routes::get_goose_config).post(routes::set_goose_config),
        )
        .route("/spaces", get(routes::list_spaces).post(routes::create_space))
        .route("/spaces/{id}/timeline", get(routes::timeline))
        .route("/spaces/{id}/pending", get(routes::list_pending))
        .route(
            "/spaces/{id}/tasks",
            get(routes::list_tasks).post(routes::create_task),
        )
        .route("/spaces/{id}/runs", get(routes::list_runs))
        .route("/mcp", post(mcp::mcp))
        .route("/spaces/{id}/run", post(routes::start_run))
        .route("/spaces/{id}/subscribe", get(routes::subscribe))
        .route(
            "/spaces/{id}/decisions",
            get(routes::list_decisions).post(routes::record_decision),
        )
        .route(
            "/spaces/{id}/reminders",
            get(routes::list_reminders).post(routes::create_reminder),
        )
        .route(
            "/spaces/{id}/calendar",
            get(routes::list_calendar).post(routes::upsert_calendar),
        )
        .route("/approvals/{id}/decision", post(routes::decide))
        .route("/reminders/{id}/dismiss", post(routes::dismiss_reminder))
        .route(
            "/integrations",
            get(routes::list_integrations).post(routes::create_integration),
        )
        .route("/integrations/{id}/ingress", post(routes::ingress))
        .route(
            "/workflows",
            get(routes::list_workflows).post(routes::create_workflow),
        )
        .route("/workflows/{id}/enable", post(routes::enable_workflow))
        .route("/workflows/{id}/disable", post(routes::disable_workflow))
        .route("/workflows/{id}/fire", post(routes::fire_workflow))
        .route("/groups", post(routes::create_group))
        .route("/groups/{id}/members", post(routes::add_group_member))
        .route(
            "/spaces/{id}/members",
            get(routes::list_members).post(routes::grant_membership),
        )
        .route(
            "/feeds/{integration_id}/visibility",
            post(routes::set_feed_visibility),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_bearer,
        ))
        .layer(cors)
        .with_state(state)
}

/// Open the store, build the router, and serve until the process is
/// signalled. `token` gates every route (SCOPE.md R4); `None` is the
/// dev default with auth disabled.
pub async fn serve(
    addr: SocketAddr,
    db_url: &str,
    goose_url: String,
    token: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::connect(db_url).await?;
    let state = AppState::new(store, goose_url, token);

    // Bring goose up under the saved provider at boot. A missing provider
    // is expected on first run (the operator configures one from the UI),
    // so it is logged, not fatal; a real launch failure is also non-fatal
    // so the server still serves the settings endpoints to fix it.
    match state.goose().restart().await {
        Ok(()) => tracing::info!("goose serve started under saved provider"),
        Err(e) => tracing::warn!(%e, "goose not started; configure a provider in settings"),
    }

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "lazyboy-server listening");
    axum::serve(listener, router(state)).await?;
    Ok(())
}
