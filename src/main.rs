mod config;
mod error;
mod handlers;
mod middleware;
mod routes;
mod state;
mod storage;

use anyhow::{Context, Result};

use crate::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env().with_context(|| "failed to load config")?;
    let addr = format!("0.0.0.0:{}", config.port);
    let state = AppState {
        file_name: config.file_name,
        auth_token: config.auth_token,
    };

    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    axum::serve(listener, app)
        .await
        .with_context(|| "server error")?;

    Ok(())
}
