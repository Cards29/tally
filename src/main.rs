use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use tally::{config::Config, routes, storage::users};

/// Logs button-press timestamps sent from a phone.
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Runs the web server. This is the default when no command is given.
    Serve,
    /// Creates an admin user.
    CreateAdmin {
        /// Login name: 3-30 of a-z, 0-9, _ and .
        #[arg(long)]
        handle: String,
        /// Name shown in the app.
        #[arg(long)]
        display_name: String,
    },
    /// Creates a device token for a user and prints it. It is never shown again.
    CreateDeviceToken {
        /// Handle of the user the token belongs to.
        #[arg(long)]
        handle: String,
        /// Device name, e.g. "Pixel shortcut".
        #[arg(long)]
        name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::from_env().with_context(|| "failed to load config")?;

    match cli.command.unwrap_or(Command::Serve) {
        Command::Serve => serve(config).await,
        Command::CreateAdmin {
            handle,
            display_name,
        } => {
            users::create_admin(&config.state.pool, &handle, &display_name).await?;
            println!("created admin {handle}");
            Ok(())
        }
        Command::CreateDeviceToken { handle, name } => {
            let token = users::create_device_token(&config.state.pool, &handle, &name).await?;
            println!("{token}");
            Ok(())
        }
    }
}

/// Applies pending migrations, then serves the app until it fails.
async fn serve(config: Config) -> Result<()> {
    sqlx::migrate!()
        .run(&config.state.pool)
        .await
        .with_context(|| "failed to run migrations")?;

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
