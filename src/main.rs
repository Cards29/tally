use chrono::{DateTime, Local};
use std::{fs, io};

use axum::{Router, routing::get};

const FILE_PATH: &str = "src/dates.log";

fn current_time() -> String {
    let local_time: DateTime<Local> = Local::now();

    let local_time = local_time.format("%a, %b %d %Y %I:%M:%S %p\n").to_string();
    println!("{}", local_time);

    let local_time = local_time.to_string();
    local_time
}

fn log_time() -> io::Result<String> {
    let mut contents =
        fs::read_to_string(FILE_PATH).expect("Should have been able to read the file");

    let entry = current_time();
    contents.push_str(&entry);

    fs::write(FILE_PATH, contents).expect("Should write on the file");
    Ok(entry)
}

fn clear_log() {
    fs::write(FILE_PATH, "").expect("Should clear the file");
}

fn do_log_time() -> String {
    match log_time() {
        Ok(entry) => format!("Entry logged: {entry}"),
        Err(e) => format!("Error: {e}"),
    }
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { do_log_time() }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app)
        .await
        .expect("I expect the server to run");
}
