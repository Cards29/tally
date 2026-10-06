use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    response::Response,
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

#[track_caller]
fn request(method: Method, uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    builder
        .body(Body::empty())
        .expect("request should be buildable")
}

async fn body_text(response: Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");

    String::from_utf8(bytes.to_vec()).expect("response body should be utf-8")
}

#[tokio::test]
async fn protected_routes_reject_missing_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    for (method, uri) in PROTECTED {
        let response = app
            .clone()
            .oneshot(request(method.clone(), uri, None))
            .await
            .expect("router should respond");

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        )
    }
}

#[tokio::test]
async fn protected_routes_reject_wrong_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    for (method, uri) in PROTECTED {
        let response = app
            .clone()
            .oneshot(request(method.clone(), uri, Some("wrong-token")))
            .await
            .expect("router should respond");

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "{method} {uri}"
        );
    }
}

#[tokio::test]
async fn log_lifecycle_with_valid_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    // Add two entries. Keep the returned text instead of guessing timestamps.
    let response = app
        .clone()
        .oneshot(request(Method::POST, "/log", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let first = body_text(response).await;

    let response = app
        .clone()
        .oneshot(request(Method::POST, "/log", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::OK);
    let second = body_text(response).await;

    // The full log holds both entries, in order.
    let response = app
        .clone()
        .oneshot(request(Method::GET, "/log", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, format!("{first}\n{second}\n"));

    // The last entry is the second one.
    let response = app
        .clone()
        .oneshot(request(Method::GET, "/log/last", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, second);

    // Removing the last entry leaves only the first.
    let response = app
        .clone()
        .oneshot(request(Method::DELETE, "/log/last", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .clone()
        .oneshot(request(Method::GET, "/log", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(body_text(response).await, format!("{first}\n"));

    // Clearing the log leaves it empty.
    let response = app
        .clone()
        .oneshot(request(Method::DELETE, "/log", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .oneshot(request(Method::GET, "/log", Some(TOKEN)))
        .await
        .expect("router should respond");
    assert_eq!(body_text(response).await, "");
}

#[tokio::test]
async fn last_entry_on_empty_log_returns_ok_with_empty_body() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    let response = app
        .oneshot(request(Method::GET, "/log/last", Some(TOKEN)))
        .await
        .expect("router should respond");

    // Current behavior. Planned to become 404; update this test when it does.
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_text(response).await, "");
}

#[tokio::test]
async fn health_returns_ok_without_token() {
    let (_dir, path) = temp_log();
    let app = app(&path);

    let response = app
        .oneshot(request(Method::GET, "/health", None))
        .await
        .expect("router should respond");

    assert_eq!(response.status(), StatusCode::OK);
}
