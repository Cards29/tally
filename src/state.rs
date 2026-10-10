use sqlx::PgPool;

use crate::storage::log_store::LogStore;

/// Shared state that every handler and the auth middleware receive.
#[derive(Clone)]
pub struct AppState {
    /// Database pool. Users and sessions always live here, whatever `log` is.
    pub pool: PgPool,
    /// Where log entries are stored.
    pub log: LogStore,
}
