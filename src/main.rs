use anyhow::{Context, Result};

use tally::{config::Config, routes};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env().with_context(|| "failed to load config")?;
    let addr = format!("0.0.0.0:{}", config.port);

    let app = routes::router(config.state);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    axum::serve(listener, app)
        .await
        .with_context(|| "server error")?;

    Ok(())
}
