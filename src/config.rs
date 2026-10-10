use std::env;

use anyhow::{Context, Result, bail};
use sqlx::PgPool;

use crate::{state::AppState, storage::log_store::LogStore};

/// Runtime settings read from environment variables.
pub struct Config {
    /// State shared with handlers and middleware.
    pub state: AppState,
    /// TCP port to listen on. Defaults to 3000.
    pub port: u16,
}

impl Config {
    /// Loads `.env` if present, then builds a `Config` from `DATABASE_URL`,
    /// `STORAGE`, `FILE_NAME` and `PORT`.
    ///
    /// Does not connect to the database yet: the pool opens connections on first use.
    ///
    /// # Errors
    /// Returns an error if `DATABASE_URL` is unset or invalid, `STORAGE` is not
    /// `postgres` or `file`, `FILE_NAME` is unset when `STORAGE=file`, or `PORT`
    /// is not a valid `u16`.
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL").with_context(|| "DATABASE_URL must be set")?;
        let pool = PgPool::connect_lazy(&database_url)
            .with_context(|| "DATABASE_URL must be a valid postgres URL")?;
        let storage = env::var("STORAGE").unwrap_or_else(|_| "postgres".to_string());
        let log = match storage.as_str() {
            "postgres" => LogStore::Postgres(pool.clone()),
            "file" => LogStore::File(
                env::var("FILE_NAME").with_context(|| "FILE_NAME must be set when STORAGE=file")?,
            ),
            other => bail!("STORAGE must be postgres or file, got {other:?}"),
        };
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .with_context(|| "PORT must be a number between 0 and 65535")?;

        Ok(Self {
            state: AppState { pool, log },
            port,
        })
    }
}
