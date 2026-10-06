use axum::http::StatusCode;

/// Liveness check. Always returns 200.
pub async fn check() -> StatusCode {
    StatusCode::OK
}
