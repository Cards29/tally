use std::fs;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use tempfile::TempDir;
use tower::ServiceExt;

use tally::{routes, state::AppState};

const TOKEN: &str = "test-token";

const PROTECTED: [(Method, &str); 5] = [
    (Method::POST, "/log"),
    (Method::GET, "/log"),
    (Method::DELETE, "/log"),
    (Method::GET, "/log/last"),
    (Method::DELETE, "/log/last"),
];

#[track_caller]
fn temp_log() -> (TempDir, String) {
    let dir = TempDir::new().expect("temp dir should be creatable");
    let path = dir
        .path()
        .join("test.log")
        .to_str()
        .expect("temp path should be valid utf-8")
        .to_string();
    (dir, path)
}

fn app(file_name: &str) -> Router {
    routes::router(AppState {
        file_name: file_name.to_string(),
        auth_token: TOKEN.to_string(),
    })
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
) -> (StatusCode, String) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = request
        .body(Body::empty())
        .expect("request should be buildable");

    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router should respond");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let body = String::from_utf8(bytes.to_vec()).expect("response body should be utf-8");

    (status, body)
}

#[tokio::test]
async fn protected_routes_reject_missing_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    for (method, uri) in PROTECTED {
        let (status, _) = send(&app, method.clone(), uri, None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[tokio::test]
async fn protected_routes_reject_wrong_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    for (method, uri) in PROTECTED {
        let (status, _) = send(&app, method.clone(), uri, Some("wrong-token")).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[tokio::test]
async fn log_lifecycle_with_valid_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    // Add two entries. Keep the returned text instead of guessing timestamps.
    let (status, first) = send(&app, Method::POST, "/log", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, second) = send(&app, Method::POST, "/log", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);

    // The full log holds both entries, in order.
    let (status, log) = send(&app, Method::GET, "/log", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, format!("{first}\n{second}\n"));

    // The last entry is the second one.
    let (status, last) = send(&app, Method::GET, "/log/last", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(last, second);

    // Removing the last entry leaves only the first.
    let (status, _) = send(&app, Method::DELETE, "/log/last", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, log) = send(&app, Method::GET, "/log", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, format!("{first}\n"));

    // Clearing the log leaves it empty.
    let (status, _) = send(&app, Method::DELETE, "/log", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, log) = send(&app, Method::GET, "/log", Some(TOKEN)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, "");
}

#[tokio::test]
async fn last_entry_on_empty_log_returns_ok_with_empty_body() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    let (status, body) = send(&app, Method::GET, "/log/last", Some(TOKEN)).await;

    // Current behavior. Planned to become 404; update this test when it does.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "");
}

#[tokio::test]
async fn health_returns_ok_without_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    let (status, _) = send(&app, Method::GET, "/health", None).await;

    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn storage_error_returns_internal_server_error() {
    let (_dir, path) = temp_log();
    // A directory at the log path can't be read as a file, so storage fails.
    fs::create_dir(&path).expect("directory should be creatable at log path");
    let app = app(&path);

    let (status, body) = send(&app, Method::GET, "/log", Some(TOKEN)).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    // Internal details stay in the server log, never in the response.
    assert_eq!(body, "");
}
