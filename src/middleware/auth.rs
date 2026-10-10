use anyhow::Context;
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{TimeDelta, Utc};
use uuid::Uuid;

use crate::{error::AppError, state::AppState, storage::tokens};

/// The user a request's token belongs to. `require_token` adds it to the
/// request, and handlers read it with `Extension<CurrentUser>`.
#[derive(Clone)]
pub struct CurrentUser {
    pub id: Uuid,
}

/// Rejects the request with 401 unless `Authorization: Bearer <token>` matches
/// an unexpired session. On success, adds the session's `CurrentUser` to the request.
///
/// Only the token's hash is looked up, so the raw token is never compared or stored.
///
/// # Errors
/// Returns a 500 if the database can't be queried.
pub async fn require_token(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let Some(token) = token else {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    };

    let session = sqlx::query!(
        "select id, user_id, last_used_at from sessions \
         where token_hash = $1 and (expires_at is null or expires_at > now())",
        tokens::hash(token)
    )
    .fetch_optional(&state.pool)
    .await
    .with_context(|| "failed to look up session")?;
    let Some(session) = session else {
        return Ok(StatusCode::UNAUTHORIZED.into_response());
    };

    // Write at most once an hour per session, not on every request.
    if Utc::now() - session.last_used_at > TimeDelta::hours(1) {
        sqlx::query!(
            "update sessions set last_used_at = now() where id = $1",
            session.id
        )
        .execute(&state.pool)
        .await
        .with_context(|| format!("failed to update last_used_at for session {}", session.id))?;
        sqlx::query!(
            "update users set last_seen_at = now() where id = $1",
            session.user_id
        )
        .execute(&state.pool)
        .await
        .with_context(|| format!("failed to update last_seen_at for user {}", session.user_id))?;
    }

    request.extensions_mut().insert(CurrentUser {
        id: session.user_id,
    });
    Ok(next.run(request).await)
}
