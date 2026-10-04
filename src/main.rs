use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;

use anyhow::{Context, Result};
use axum::{Router, extract::State, routing::get};
use chrono::{DateTime, Local};

#[derive(Clone)]
struct AppState {
    file_name: String,
}

fn current_time() -> String {
    let local_time: DateTime<Local> = Local::now();

    let local_time = local_time.format("%a, %b %d %Y %I:%M:%S %p").to_string();
    println!("Printing from current_time function {}", local_time);

    local_time
}

fn log_time(file_name: &str) -> Result<String> {
    let entry = current_time();
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_name)
        .with_context(|| format!("failed to open {}", file_name))?;

    writeln!(file, "{}", entry).with_context(|| format!("failed to write to {}", file_name))?;
    Ok(entry)
}

fn clear_log(file_name: &str) -> Result<()> {
    fs::write(file_name, "").with_context(|| "failed to clear the log")?;
    Ok(())
}

async fn log_handler(State(state): State<AppState>) -> String {
    match log_time(&state.file_name) {
        Ok(entry) => format!("Entry logged: {entry}"),
        Err(e) => format!("Error logging entry: {e}"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let state = AppState {
        file_name: env::var("FILE_NAME").with_context(|| "FILE_NAME must be set")?,
    };
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");

    let app = Router::new().route("/", get(log_handler)).with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    axum::serve(listener, app)
        .await
        .with_context(|| "server error")?;

    Ok(())
}
