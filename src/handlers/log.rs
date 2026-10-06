use axum::{extract::State, http::StatusCode};

use crate::{error::AppError, state::AppState, storage::log as store};

/// `POST /log`: appends the current time and returns the new entry.
pub async fn add_entry(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::log_time(&state.file_name)?)
}

/// `GET /log`: returns the whole log, one entry per line.
pub async fn show_log(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::read_log(&state.file_name)?)
}

/// `GET /log/last`: returns the last entry, or an empty body if the log is empty.
pub async fn show_last(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::last_entry(&state.file_name)?)
}

/// `DELETE /log/last`: removes the last entry and returns 204.
pub async fn clear_last(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_last_entry(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /log`: empties the log and returns 204.
pub async fn clear_all(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_log(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}
