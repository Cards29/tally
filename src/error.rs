use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Handler error that wraps any `anyhow::Error` and becomes a 500 response.
pub struct AppError(anyhow::Error);

/// Logs the full error chain to stderr and returns a bare 500.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("{:#}", self.0);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
