/// Shared state that every handler and the auth middleware receive.
#[derive(Clone)]
pub struct AppState {
    /// Path of the log file.
    pub file_name: String,
    /// Bearer token that protected routes require.
    pub auth_token: String,
}
