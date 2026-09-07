//! Test infrastructure for the `tb_axum` crate.
//!
//! Builds the production router stack without a database: the pool is lazy and points at
//! `127.0.0.1:1`, so any handler that touches the database fails fast with a connection
//! refused error, and sessions are backed by a `MemoryStore` that tests can seed.

use std::sync::{Arc, Once};

use axum::Router;
use http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use tb_sqlx::DbPool;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

use crate::appstate::AppState;
use crate::routes;
use crate::strava::RequestSession;

const SESSION_KEY: &str = "session";

/// Dummies the OAuth client environment variables so the `STRAVACLIENT` lazy static
/// does not panic when it is first used in a test.
fn set_oauth_env_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        std::env::set_var("CLIENT_ID", "test-client-id");
        std::env::set_var("CLIENT_SECRET", "test-client-secret");
    });
}

/// Builds the production routes with a lazy pool (port 1 -> instant connection refused)
/// and a `MemoryStore` session layer.
pub(crate) fn test_app(store: MemoryStore) -> Router {
    set_oauth_env_once();
    let session_layer = SessionManagerLayer::new(store);
    let pool = DbPool::lazy("postgres://127.0.0.1:1/test");
    routes(AppState::new(pool)).layer(session_layer)
}

/// Creates a session in `store` containing `value` and returns the cookie header value
/// that authenticates with it.
pub(crate) async fn cookie_for(store: &MemoryStore, value: RequestSession) -> String {
    let session = Session::new(None, Arc::new(store.clone()), None);
    session
        .insert(SESSION_KEY, &value)
        .await
        .expect("session insert");
    session.save().await.expect("session save");
    let id = session.id().expect("session id");
    format!("id={id}")
}

/// Cookie header value for a regular (non-admin) user.
pub(crate) async fn user_cookie(store: &MemoryStore) -> String {
    cookie_for(store, RequestSession::new_dummy(false)).await
}

/// Cookie header value for an admin user.
pub(crate) async fn admin_cookie(store: &MemoryStore) -> String {
    cookie_for(store, RequestSession::new_dummy(true)).await
}

/// Runs a single request against `app` and returns its status, headers and body.
pub(crate) async fn run(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut req = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "localhost")
        .body(axum::body::Body::empty())
        .expect("valid request");
    if let Some(cookie) = cookie {
        req.headers_mut()
            .insert(header::COOKIE, cookie.parse().expect("valid cookie"));
    }
    let res = app
        .oneshot(req)
        .await
        .expect("oneshot request should succeed");
    let status = res.status();
    let headers = res.headers().clone();
    let body = res.into_body().collect().await.expect("body").to_bytes();
    (status, headers, body.to_vec())
}
