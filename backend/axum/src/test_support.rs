//! Test infrastructure for the `tb_axum` crate.
//!
//! Builds the production router stack without a database: the state carries
//! a `mockall`-generated `MockServiceApi` (the suite's `ServiceApi` adapter,
//! spec #558 §8) and sessions are backed by a `MemoryStore` that tests can
//! seed.

#![allow(clippy::too_many_arguments)]

use std::sync::{Arc, Once};

use axum::Router;
use http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use tb_exec::ServiceApi;
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

/// Builds the production routes over the service API and a `MemoryStore`
/// session layer.
pub(crate) fn test_app(store: MemoryStore, api: Arc<dyn ServiceApi>) -> Router {
    set_oauth_env_once();
    let session_layer = SessionManagerLayer::new(store);
    routes(AppState::new(api)).layer(session_layer)
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
    run_with_body(app, method, uri, cookie, None).await
}

/// Runs a single request with a JSON body against `app`.
pub(crate) async fn run_json(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    body: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    run_with_body(app, method, uri, cookie, Some(body)).await
}

async fn run_with_body(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    json: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    match json {
        Some(json) => {
            run_raw(
                app,
                method,
                uri,
                cookie,
                json.as_bytes(),
                "application/json",
            )
            .await
        }
        None => run_raw(app, method, uri, cookie, &[], "").await,
    }
}

/// Runs a single request with a raw body and an explicit content type
/// (the multipart uploads' shape — `run_json` cannot carry one).
pub(crate) async fn run_raw(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    body: &[u8],
    content_type: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "localhost");
    if !content_type.is_empty() {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    let mut req = builder
        .body(axum::body::Body::from(body.to_vec()))
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

/// can read it frame by frame (collecting it would hang on a live stream).
pub(crate) async fn run_sse(
    app: Router,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, axum::body::Body) {
    let builder = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::HOST, "localhost");
    let mut req = builder
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
    let body = res.into_body();
    (status, headers, body)
}

/// Reads up to `count` frames off an SSE body, each within `timeout` (a live
/// stream never ends, so a per-frame timeout bounds the read). Returns the
/// frames as lossy UTF-8 strings.
pub(crate) async fn read_sse_frames(
    body: &mut axum::body::Body,
    count: usize,
    timeout: std::time::Duration,
) -> Vec<String> {
    use http_body_util::BodyExt;
    let mut frames = Vec::new();
    for _ in 0..count {
        match tokio::time::timeout(timeout, body.frame()).await {
            Ok(Some(Ok(frame))) => {
                if let Ok(chunk) = frame.into_data() {
                    frames.push(String::from_utf8_lossy(&chunk).to_string());
                }
            }
            _ => break,
        }
    }
    frames
}
