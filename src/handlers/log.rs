use axum::{Extension, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};

use crate::{error::AppError, middleware::auth::CurrentUser, state::AppState};

const TIME_FORMAT: &str = "%a, %b %d %Y %H:%M:%S UTC";

/// Formats an entry for the response, e.g. `Mon, Oct 06 2026 08:03:09 UTC`.
fn format_entry(time: DateTime<Utc>) -> String {
    time.format(TIME_FORMAT).to_string()
}

/// `POST /log`: adds an entry stamped with the current time and returns it.
pub async fn add_entry(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<String, AppError> {
    Ok(format_entry(state.log.add_entry(user.id).await?))
}

/// `GET /log`: returns the whole log, one entry per line.
pub async fn show_log(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<String, AppError> {
    let entries = state.log.show_log(user.id).await?;
    Ok(entries
        .into_iter()
        .map(|t| format_entry(t) + "\n")
        .collect())
}

/// `GET /log/last`: returns the last entry, or an empty body if the log is empty.
pub async fn show_last(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<String, AppError> {
    Ok(state
        .log
        .show_last(user.id)
        .await?
        .map(format_entry)
        .unwrap_or_default())
}

/// `DELETE /log/last`: removes the last entry and returns 204.
pub async fn clear_last(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<StatusCode, AppError> {
    state.log.clear_last(user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /log`: empties the log and returns 204.
pub async fn clear_all(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<StatusCode, AppError> {
    state.log.clear_all(user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
