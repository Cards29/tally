use std::env;

use anyhow::{Context, Result};

use crate::state::AppState;

/// Runtime settings read from environment variables.
pub struct Config {
    /// State shared with handlers and middleware.
    pub state: AppState,
    /// TCP port to listen on. Defaults to 3000.
    pub port: u16,
}

impl Config {
    /// Loads `.env` if present, then builds a `Config` from `FILE_NAME`, `AUTH_TOKEN` and `PORT`.
    ///
    /// # Errors
    /// Returns an error if `FILE_NAME` or `AUTH_TOKEN` is unset, or if `PORT` is not a valid `u16`.
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let file_name = env::var("FILE_NAME").with_context(|| "FILE_NAME must be set")?;
        let auth_token = env::var("AUTH_TOKEN").with_context(|| "AUTH_TOKEN must be set")?;
        let port: u16 = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .with_context(|| "PORT must be a number between 0 and 65535")?;

        Ok(Self {
            state: AppState {
                file_name,
                auth_token,
            },
            port,
        })
    }
}
