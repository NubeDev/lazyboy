use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// The error surface the HTTP layer renders. Store/core/bridge faults
/// collapse to 500 with a stable JSON body; the auth and not-found cases
/// carry their own status so the UI can branch on them.
#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    NotFound(String),
    Store(lazyboy_store::StoreError),
    Core(lazyboy_core::CoreError),
    Bridge(lazyboy_bridge::BridgeError),
}

impl From<lazyboy_store::StoreError> for ApiError {
    fn from(e: lazyboy_store::StoreError) -> Self {
        // A store lookup miss is a client-addressable 404, not a 500.
        match e {
            lazyboy_store::StoreError::NotFound(what) => ApiError::NotFound(what),
            other => ApiError::Store(other),
        }
    }
}

impl From<lazyboy_core::CoreError> for ApiError {
    fn from(e: lazyboy_core::CoreError) -> Self {
        ApiError::Core(e)
    }
}

impl From<lazyboy_bridge::BridgeError> for ApiError {
    fn from(e: lazyboy_bridge::BridgeError) -> Self {
        ApiError::Bridge(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_owned()),
            ApiError::NotFound(what) => (StatusCode::NOT_FOUND, what),
            ApiError::Store(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::Core(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::Bridge(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}
