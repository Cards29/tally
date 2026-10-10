use std::fs;

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use sqlx::PgPool;
use tempfile::TempDir;
use tower::ServiceExt;

use tally::{
    routes,
    state::AppState,
    storage::{log_store::LogStore, users},
};

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

/// Builds the app with entries stored in Postgres.
fn app(pool: &PgPool) -> Router {
    routes::router(AppState {
        pool: pool.clone(),
        log: LogStore::Postgres(pool.clone()),
    })
}

/// Creates a user with this handle and returns a device token for them.
async fn user_token(pool: &PgPool, handle: &str) -> String {
    users::create_admin(pool, handle, "Test")
        .await
        .expect("user should be created");
    users::create_device_token(pool, handle, "test device")
        .await
        .expect("token should be created")
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

#[sqlx::test]
async fn protected_routes_reject_missing_token(pool: PgPool) {
    let app = app(&pool);

    for (method, uri) in PROTECTED {
        let (status, _) = send(&app, method.clone(), uri, None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[sqlx::test]
async fn protected_routes_reject_wrong_token(pool: PgPool) {
    // A real session exists, so the wrong token fails the lookup, not an empty table.
    let _ = user_token(&pool, "alice").await;
    let app = app(&pool);

    for (method, uri) in PROTECTED {
        let (status, _) = send(&app, method.clone(), uri, Some("tly_wrong")).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {uri}");
    }
}

#[sqlx::test]
async fn log_lifecycle_with_valid_token(pool: PgPool) {
    let token = user_token(&pool, "alice").await;
    let app = app(&pool);

    // Add two entries. Keep the returned text instead of guessing timestamps.
    let (status, first) = send(&app, Method::POST, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    chrono::NaiveDateTime::parse_from_str(&first, "%a, %b %d %Y %H:%M:%S UTC")
        .expect("entry should match the response format");

    let (status, second) = send(&app, Method::POST, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);

    // The full log holds both entries, in order.
    let (status, log) = send(&app, Method::GET, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, format!("{first}\n{second}\n"));

    // The last entry is the second one.
    let (status, last) = send(&app, Method::GET, "/log/last", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(last, second);

    // Removing the last entry leaves only the first.
    let (status, _) = send(&app, Method::DELETE, "/log/last", Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, log) = send(&app, Method::GET, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, format!("{first}\n"));

    // Clearing the log leaves it empty.
    let (status, _) = send(&app, Method::DELETE, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, log) = send(&app, Method::GET, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, "");
}

#[sqlx::test]
async fn users_only_see_their_own_entries(pool: PgPool) {
    let alice = user_token(&pool, "alice").await;
    let bob = user_token(&pool, "bob").await;
    let app = app(&pool);

    let (status, entry) = send(&app, Method::POST, "/log", Some(&alice)).await;
    assert_eq!(status, StatusCode::OK);

    let (status, log) = send(&app, Method::GET, "/log", Some(&bob)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, "");
    let (status, log) = send(&app, Method::GET, "/log", Some(&alice)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(log, format!("{entry}\n"));
}

#[sqlx::test]
async fn activity_is_recorded_at_most_once_an_hour(pool: PgPool) {
    let token = user_token(&pool, "alice").await;
    let app = app(&pool);

    // A session used 30 minutes ago is left alone.
    let used = sqlx::query_scalar!(
        "update sessions set last_used_at = now() - interval '30 minutes' returning last_used_at"
    )
    .fetch_one(&pool)
    .await
    .expect("session should be updatable");
    let (status, _) = send(&app, Method::GET, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let after = sqlx::query_scalar!("select last_used_at from sessions")
        .fetch_one(&pool)
        .await
        .expect("session should be readable");
    assert_eq!(after, used);

    // A session used 2 hours ago is updated, and so is its user.
    let used = sqlx::query_scalar!(
        "update sessions set last_used_at = now() - interval '2 hours' returning last_used_at"
    )
    .fetch_one(&pool)
    .await
    .expect("session should be updatable");
    let seen = sqlx::query_scalar!(
        "update users set last_seen_at = now() - interval '2 hours' returning last_seen_at"
    )
    .fetch_one(&pool)
    .await
    .expect("user should be updatable");
    let (status, _) = send(&app, Method::GET, "/log", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    let after = sqlx::query_scalar!("select last_used_at from sessions")
        .fetch_one(&pool)
        .await
        .expect("session should be readable");
    assert!(
        after > used,
        "last_used_at should move forward, got {after}"
    );
    let after = sqlx::query_scalar!("select last_seen_at from users")
        .fetch_one(&pool)
        .await
        .expect("user should be readable");
    assert!(
        after > seen,
        "last_seen_at should move forward, got {after}"
    );
}

#[sqlx::test]
async fn last_entry_on_empty_log_returns_ok_with_empty_body(pool: PgPool) {
    let token = user_token(&pool, "alice").await;
    let app = app(&pool);

    let (status, body) = send(&app, Method::GET, "/log/last", Some(&token)).await;

    // Current behavior. Planned to become 404; update this test when it does.
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "");
}

#[sqlx::test]
async fn health_returns_ok_without_token(pool: PgPool) {
    let app = app(&pool);

    let (status, _) = send(&app, Method::GET, "/health", None).await;

    assert_eq!(status, StatusCode::OK);
}

#[sqlx::test]
async fn storage_error_returns_internal_server_error(pool: PgPool) {
    let token = user_token(&pool, "alice").await;
    let (_dir, path) = temp_log();
    // A directory at the log path can't be read as a file, so storage fails.
    fs::create_dir(&path).expect("directory should be creatable at log path");
    let app = routes::router(AppState {
        pool,
        log: LogStore::File(path),
    });

    let (status, body) = send(&app, Method::GET, "/log", Some(&token)).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    // Internal details stay in the server log, never in the response.
    assert_eq!(body, "");
}
