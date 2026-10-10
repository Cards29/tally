use axum::{extract::State, http::StatusCode};
use chrono::{DateTime, Utc};

use crate::{error::AppError, state::AppState, storage::log as store};

const TIME_FORMAT: &str = "%a, %b %d %Y %H:%M:%S UTC";

/// Formats an entry for the response, e.g. `Mon, Oct 06 2026 08:03:09 UTC`.
fn format_entry(time: DateTime<Utc>) -> String {
    time.format(TIME_FORMAT).to_string()
}

/// `POST /log`: appends the current time and returns the new entry.
pub async fn add_entry(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(format_entry(store::add_entry(&state.file_name)?))
}

/// `GET /log`: returns the whole log, one entry per line.
pub async fn show_log(State(state): State<AppState>) -> Result<String, AppError> {
    let entries = store::show_log(&state.file_name)?;
    Ok(entries
        .into_iter()
        .map(|t| format_entry(t) + "\n")
        .collect())
}

/// `GET /log/last`: returns the last entry, or an empty body if the log is empty.
pub async fn show_last(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::show_last(&state.file_name)?
        .map(format_entry)
        .unwrap_or_default())
}

/// `DELETE /log/last`: removes the last entry and returns 204.
pub async fn clear_last(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_last(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /log`: empties the log and returns 204.
pub async fn clear_all(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_all(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}
