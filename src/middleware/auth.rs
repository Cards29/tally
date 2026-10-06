use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

use crate::state::AppState;

/// Rejects the request with 401 unless `Authorization: Bearer <token>` matches `AUTH_TOKEN`.
///
/// The comparison runs in constant time, so response timing does not leak the token.
pub async fn require_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));

    let authorized = match token {
        Some(token) => bool::from(token.as_bytes().ct_eq(state.auth_token.as_bytes())),
        None => false,
    };

    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    next.run(request).await
}
