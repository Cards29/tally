use axum::{extract::State, http::StatusCode};

use crate::{error::AppError, state::AppState, storage::log as store};

pub async fn add_entry(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::log_time(&state.file_name)?)
}

pub async fn show_log(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::read_log(&state.file_name)?)
}

pub async fn show_last(State(state): State<AppState>) -> Result<String, AppError> {
    Ok(store::last_entry(&state.file_name)?)
}

pub async fn clear_last(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_last_entry(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn clear_all(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    store::clear_log(&state.file_name)?;
    Ok(StatusCode::NO_CONTENT)
}
