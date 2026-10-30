//! The axum contract suite (spec #558 §8): pure HTTP contract tests against
//! a `mockall`-generated `MockServiceApi` — no loop, no in-memory store,
//! no executor tokio plumbing.
//!
//! The pre-agreed seam (spec #558 §8, the coverage split's axum row): every
//! route — status codes, JSON bodies, `ReadOutcome`/`Err` mapping, the SSE
//! wire shape, session/cookie — asserted here. The mock's expectations
//! (`.times(n)` + arg predicates) verify the handler called the right
//! operation with the right args; the canned replies are the mock's; the
//! error mapping (404/500/401/403/400/409/429) is the mock returning
//! `Err(...)` directly — no failing `begin` anywhere.
//!
//! What this suite deliberately does not cover (the split's other rows):
//! loop + real-store truth (reads-as-messages, FIFO, rollback, reap, queue
//! drain, Registry-level lifecycle, the direct ops' transactions) lives in
//! `backend/sqlx/tests/executor_seam.rs` on real Postgres; the read/write
//! dispatch logic lives in the domain suite (`apiread_tests` beside
//! `apiwrite_tests`). The executor-lifecycle tests (an attached stream
//! keeps the executor alive; a stream survives the executor's reap; a wake
//! never spawns) live there too.

use std::sync::{Arc, Once};
use std::time::Duration;

use axum::Router;
use http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use tower_sessions::{MemoryStore, Session, SessionManagerLayer};

use tb_domain::{
    ActTypeId, Activity, ActivityId, ApiRead, DescendReport, OnboardingStatus, Part, PartId,
    PartNote, PartNoteId, PartTypeId, ReadOutcome, Service, ServicePlan, Session as TbSession,
    Shop, ShopId, ShopSubscription, ShopSubscriptionWithDetails, SubscriptionId,
    SubscriptionStatus, Summary, TbResult, UsageId, User, UserId, UserPublic, UserState,
    WriteOutcome,
};
use tb_exec::{ExecutorStatus, MockServiceApi, ServiceApi};
use time::macros::datetime;
use tower::ServiceExt;

use crate::appstate::AppState;
use crate::routes;
use crate::strava::RequestSession;

// ─── Test infrastructure ──────────────────────────────────────────────────

const SESSION_KEY: &str = "session";

/// Dummies the OAuth client environment variables so the `STRAVACLIENT` lazy
/// static does not panic when it is first used in a test.
fn set_oauth_env_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        std::env::set_var("CLIENT_ID", "test-client-id");
        std::env::set_var("CLIENT_SECRET", "test-client-secret");
    });
}

/// Builds the production routes over the service API and a `MemoryStore`
/// session layer.
fn test_app(store: MemoryStore, api: Arc<dyn ServiceApi>) -> Router {
    set_oauth_env_once();
    let session_layer = SessionManagerLayer::new(store);
    routes(AppState::new(api)).layer(session_layer)
}

/// Creates a session in `store` containing `value` and returns the cookie
/// header value that authenticates with it.
async fn cookie_for(store: &MemoryStore, value: RequestSession) -> String {
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
async fn user_cookie(store: &MemoryStore) -> String {
    cookie_for(store, RequestSession::new_dummy(false)).await
}

/// Cookie header value for an admin user.
async fn admin_cookie(store: &MemoryStore) -> String {
    cookie_for(store, RequestSession::new_dummy(true)).await
}

/// Runs a single request against `app` and returns its status, headers and
/// body.
async fn run(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    run_with_body(app, method, uri, cookie, None).await
}

/// Runs a single request with a JSON body against `app`.
async fn run_json(
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
async fn run_raw(
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

/// Runs a single request with extra request headers and an optional JSON
/// body (the `X-Active-Shop` contract's shape — `run`/`run_json` cannot
/// carry headers).
async fn run_headers(
    app: Router,
    method: Method,
    uri: &str,
    cookie: Option<&str>,
    headers: &[(&str, &str)],
    json: Option<&str>,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::HOST, "localhost");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut body = Vec::new();
    if let Some(json) = json {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        body = json.as_bytes().to_vec();
    }
    let mut req = builder
        .body(axum::body::Body::from(body))
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

/// Runs a GET request and returns the SSE body (can read it frame by frame;
/// collecting it would hang on a live stream).
async fn run_sse(
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
async fn read_sse_frames(
    body: &mut axum::body::Body,
    count: usize,
    timeout: Duration,
) -> Vec<String> {
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

// ─── Canned values (the mock's replies, the assertions' subjects) ─────────

/// The nil UUID (the Uuid-backed ids' canned value; the route paths' shape).
const NIL: &str = "00000000-0000-0000-0000-000000000000";

/// The suite's user (the dummy session's identity, id 1).
fn user() -> User {
    User {
        id: UserId::from(1),
        name: "Test User".to_string(),
        firstname: "Test".to_string(),
        avatar: None,
        is_admin: false,
        onboarding_status: OnboardingStatus::Pending,
    }
}

/// The suite's part.
fn part() -> Part {
    Part {
        id: PartId::from(1),
        owner: UserId::from(1),
        what: PartTypeId::from(1),
        name: "Chain".to_string(),
        vendor: "Shimano".to_string(),
        model: "CN-HG62".to_string(),
        purchase: datetime!(2024-01-01 00:00:00 UTC),
        last_used: datetime!(2024-01-01 00:00:00 UTC),
        disposed_at: None,
        usage: UsageId::default(),
        source: None,
        shop: None,
    }
}

/// The suite's shop.
fn shop() -> Shop {
    Shop {
        id: ShopId::from(1),
        owner: UserId::from(1),
        name: "Bike Barn".to_string(),
        description: None,
        auto_approve: false,
        created_at: datetime!(2024-01-01 00:00:00 UTC),
    }
}

/// The suite's subscription (pending, the create's shape).
fn subscription() -> ShopSubscription {
    ShopSubscription {
        id: SubscriptionId::from(1),
        shop_id: ShopId::from(1),
        user_id: UserId::from(1),
        status: SubscriptionStatus::Pending,
        message: Some("hi".to_string()),
        response_message: None,
        created_at: datetime!(2024-01-01 00:00:00 UTC),
        updated_at: datetime!(2024-01-01 00:00:00 UTC),
    }
}

/// The suite's subscription with its shop details (the read's shape).
fn subscription_with_details() -> ShopSubscriptionWithDetails {
    ShopSubscriptionWithDetails {
        id: subscription().id,
        shop_id: subscription().shop_id,
        user_id: subscription().user_id,
        status: subscription().status,
        message: subscription().message,
        response_message: subscription().response_message,
        created_at: subscription().created_at,
        updated_at: subscription().updated_at,
        shop: shop(),
    }
}

/// The suite's text note.
fn note() -> PartNote {
    PartNote {
        id: PartNoteId::from(1),
        part: PartId::from(1),
        name: "worn".to_string(),
        mime: None,
        filename: None,
        size: None,
        created: datetime!(2024-01-01 00:00:00 UTC),
    }
}

/// The suite's file note (the download route's note).
fn file_note() -> PartNote {
    PartNote {
        id: PartNoteId::from(1),
        part: PartId::from(1),
        name: "worn".to_string(),
        mime: Some("application/pdf".to_string()),
        filename: Some("chain.pdf".to_string()),
        size: Some(7),
        created: datetime!(2024-01-01 00:00:00 UTC),
    }
}

/// The suite's activity.
fn activity() -> Activity {
    Activity {
        id: ActivityId::from(1),
        user_id: UserId::from(1),
        what: ActTypeId::from(1),
        name: "Morning Ride".to_string(),
        start: datetime!(2024-01-01 08:00:00 UTC),
        duration: 3600,
        time: None,
        distance: Some(20000),
        climb: Some(100),
        descend: Some(100),
        energy: Some(5000),
        gear: None,
        device_name: None,
        external_id: None,
    }
}

/// A `Summary` with the suite's part as its one live entry (a create
/// write's frame — the `201` body's source).
fn summary_with_part() -> Summary {
    let mut summary = Summary::default();
    summary.parts.upsert(part());
    summary
}

/// A `Summary` with the suite's note as its one live entry.
fn summary_with_note() -> Summary {
    let mut summary = Summary::default();
    summary.part_notes.upsert(note());
    summary
}

/// A `Summary` with the suite's shop as its one live entry.
fn summary_with_shop() -> Summary {
    let mut summary = Summary::default();
    summary.shops.upsert(shop());
    summary
}

/// A `Summary` with the suite's service as its one live entry.
fn summary_with_service() -> Summary {
    let mut summary = Summary::default();
    summary.services.upsert(Service {
        id: tb_domain::ServiceId::default(),
        part_id: PartId::from(1),
        time: datetime!(2024-01-01 00:00:00 UTC),
        redone: datetime!(2024-01-01 00:00:00 UTC),
        name: "Drainage".to_string(),
        notes: "ok".to_string(),
        usage: UsageId::default(),
        successor: None,
        plans: vec![],
    });
    summary
}

/// A `Summary` with the suite's plan as its one live entry.
fn summary_with_plan() -> Summary {
    let mut summary = Summary::default();
    summary.plans.upsert(ServicePlan {
        id: tb_domain::ServicePlanId::default(),
        part: Some(PartId::from(1)),
        what: PartTypeId::from(1),
        hook: None,
        name: "Drainage".to_string(),
        days: None,
        hours: None,
        km: Some(5000),
        climb: None,
        descend: None,
        rides: Some(30),
        uid: None,
        energy: None,
    });
    summary
}

// ─── The fixture ──────────────────────────────────────────────────────────

/// The canned `User` read reply: the suite's user with an empty `Summary`.
fn user_state_reply() -> impl Fn(Box<dyn TbSession>, ApiRead) -> TbResult<ReadOutcome> + 'static {
    |_session, _read| {
        Ok(ReadOutcome::User(Box::new(UserState {
            user: user(),
            summary: Summary::default(),
        })))
    }
}

/// One contract test's fixture: the session store and the mock. The test
/// sets up expectations on the mock, then calls [`build_app`] to get the
/// production router.
fn mock_ctx() -> (MemoryStore, MockServiceApi) {
    (MemoryStore::default(), MockServiceApi::new())
}

/// Build the production router over the mock (call after setting up
/// expectations).
fn build_app(store: &MemoryStore, mock: MockServiceApi) -> Router {
    let api: Arc<dyn tb_exec::ServiceApi> = Arc::new(mock);
    test_app(store.clone(), api)
}

/// An unauthenticated request to an authenticated route answers `401` with
/// the login body — and never reaches the service API (the session
/// extractor rejects it first; the mock has no expectations, so any call
/// would panic).
async fn expect_unauth(method: Method, uri: &str) {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, _headers, body) = run(app, method, uri, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body, "Please login".as_bytes());
}

/// An authenticated non-admin request to an admin route answers `404`
/// (hidden, not forbidden) — and never reaches the service API.
async fn expect_hidden_from_non_admin(method: Method, uri: &str) {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, method, uri, Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─── Static routes (no service API) ───────────────────────────────────────

#[tokio::test]
async fn types_part_returns_part_types() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, _headers, body) = run(app, Method::GET, "/api/types/part", None).await;
    assert_eq!(status, StatusCode::OK);
    let types: Vec<serde_json::Value> = serde_json::from_slice(&body).expect("part types are json");
    assert!(!types.is_empty());
}

#[tokio::test]
async fn types_activity_returns_activity_types() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, _headers, body) = run(app, Method::GET, "/api/types/activity", None).await;
    assert_eq!(status, StatusCode::OK);
    let types: Vec<serde_json::Value> =
        serde_json::from_slice(&body).expect("activity types are json");
    assert!(!types.is_empty());
}

#[tokio::test]
async fn callback_validation_echoes_challenge() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, _headers, body) = run(
        app,
        Method::GET,
        "/strava/callback?hub.mode=subscribe&hub.challenge=xyz&hub.verify_token=tendabike_strava",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let hub: serde_json::Value = serde_json::from_slice(&body).expect("hub is json");
    assert_eq!(hub["hub.challenge"], "xyz");
}

#[tokio::test]
async fn callback_validation_rejects_wrong_token() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, _headers, _body) = run(
        app,
        Method::GET,
        "/strava/callback?hub.mode=subscribe&hub.challenge=xyz&hub.verify_token=wrong",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn login_redirects_to_strava_authorize() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, headers, _body) = run(app, Method::GET, "/strava/login", None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let location = headers["location"].to_str().expect("location").to_owned();
    assert!(
        location.starts_with("https://www.strava.com/oauth/authorize?"),
        "unexpected location {location}"
    );
    assert!(location.contains("client_id=test-client-id"));
    assert!(location.contains("state="));
}

#[tokio::test]
async fn token_error_redirects_to_root() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, headers, _body) = run(
        app,
        Method::GET,
        "/strava/token?error=access_denied&state=foo",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers["location"], "/");
}

#[tokio::test]
async fn token_wrong_scope_redirects_to_root() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, headers, _body) = run(
        app,
        Method::GET,
        "/strava/token?code=abc&state=foo&scope=read",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers["location"], "/");
}

#[tokio::test]
async fn logout_redirects_to_root() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, headers, _body) = run(app, Method::GET, "/strava/logout", None).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(headers["location"], "/");
}

// ─── Auth gates (401/404 before the service API) ──────────────────────────

#[tokio::test]
async fn user_requires_auth() {
    expect_unauth(Method::GET, "/api/user").await;
}

#[tokio::test]
async fn user_ping_requires_auth() {
    expect_unauth(Method::GET, "/api/user/ping").await;
}

#[tokio::test]
async fn part_requires_auth() {
    expect_unauth(Method::GET, "/api/part/categories").await;
}

#[tokio::test]
async fn stream_requires_auth() {
    expect_unauth(Method::GET, "/api/user/stream").await;
}

#[tokio::test]
async fn part_attach_requires_auth() {
    expect_unauth(Method::POST, "/api/part/attach").await;
}

#[tokio::test]
async fn service_requires_auth() {
    expect_unauth(Method::POST, "/api/service").await;
}

#[tokio::test]
async fn plan_requires_auth() {
    expect_unauth(Method::POST, "/api/plan").await;
}

#[tokio::test]
async fn activ_requires_auth() {
    expect_unauth(Method::GET, "/api/activ/1").await;
}

#[tokio::test]
async fn shop_requires_auth() {
    expect_unauth(Method::GET, "/api/shop").await;
}

#[tokio::test]
async fn partnote_list_requires_auth() {
    expect_unauth(Method::GET, "/api/part/1/notes").await;
}

#[tokio::test]
async fn partnote_create_requires_auth() {
    expect_unauth(Method::POST, "/api/part/1/notes").await;
}

#[tokio::test]
async fn partnote_file_upload_requires_auth() {
    expect_unauth(Method::POST, "/api/part/1/notes/file").await;
}

#[tokio::test]
async fn partnote_file_download_requires_auth() {
    expect_unauth(Method::GET, "/api/part/notes/1/file").await;
}

#[tokio::test]
async fn partnote_file_update_requires_auth() {
    expect_unauth(Method::PUT, "/api/part/notes/1/file").await;
}

#[tokio::test]
async fn partnote_file_remove_requires_auth() {
    expect_unauth(Method::DELETE, "/api/part/notes/1/file").await;
}

#[tokio::test]
async fn partnote_update_requires_auth() {
    expect_unauth(Method::PUT, "/api/part/notes/1").await;
}

#[tokio::test]
async fn partnote_delete_requires_auth() {
    expect_unauth(Method::DELETE, "/api/part/notes/1").await;
}

#[tokio::test]
async fn user_all_hidden_from_non_admin() {
    expect_hidden_from_non_admin(Method::GET, "/api/user/all").await;
}

#[tokio::test]
async fn activ_rescan_hidden_from_non_admin() {
    expect_hidden_from_non_admin(Method::GET, "/api/activ/rescan").await;
}

#[tokio::test]
async fn strava_sync_hidden_from_non_admin() {
    expect_hidden_from_non_admin(Method::GET, "/strava/sync?time=0").await;
}

#[tokio::test]
async fn admin_sync_requires_admin() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::GET, "/strava/sync/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn wrong_method_not_allowed() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, _headers, _body) = run(app, Method::POST, "/api/user", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn unknown_path_not_found() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let (status, _headers, _body) = run(app, Method::GET, "/api/does-not-exist", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ─── Read routes (canned `ReadOutcome` in, status + body) ─────────────────

/// `GET /api/user` runs the hydration write, then the unified `User` read
/// — two FIFO-ordered messages on the user's executor (spec #558 §4) — and
/// answers `200` with the full `UserState` (the user record plus the
/// `Summary`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_user_returns_200_and_userstate_body() {
    let (store, mut mock) = mock_ctx();
    mock.expect_hydrate_gear()
        .times(1)
        .returning(|_session| Ok(Summary::default()));
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::User(Box::new(UserState {
            user: user(),
            summary: Summary::default(),
        })))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/user", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"user":{"id":1,"name":"Test User","firstname":"Test","avatar":null,"is_admin":false,"onboarding_status":"pending"},"summary":{"activities":{},"parts":{},"attachments":{},"usages":{},"services":{},"plans":{},"part_notes":{},"shops":{},"users":{}}}"#
    );
}

/// `GET /api/user/export` rides the same unified `User` read (staying
/// non-hydrating, spec #558 §3.4/§4) and answers `200` with the export
/// composite — the `User` plus the live entities of every `Summary` map
/// (all empty for this user).
#[tokio::test]
async fn get_user_export_returns_200_and_export_body() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::User(Box::new(UserState {
            user: user(),
            summary: Summary::default(),
        })))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/user/export", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"user":{"id":1,"name":"Test User","firstname":"Test","avatar":null,"is_admin":false,"onboarding_status":"pending"},"parts":[],"attachments":[],"services":[],"plans":[],"usages":[],"activities":[],"shops":[]}"#
    );
}

/// `GET /api/user` runs the hydration write, then the unified `User` read
/// — two FIFO-ordered messages on the user's executor (spec #558 §4) — and
/// answers `200` with the full `UserState` (the user record plus the
/// `Summary`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn summary_returns_200_and_summary_body() {
    let (store, mut mock) = mock_ctx();
    mock.expect_hydrate_gear()
        .times(1)
        .returning(|_session| Ok(Summary::default()));
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::User(Box::new(UserState {
            user: user(),
            summary: Summary::default(),
        })))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/user", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"user":{"id":1,"name":"Test User","firstname":"Test","avatar":null,"is_admin":false,"onboarding_status":"pending"},"summary":{"activities":{},"parts":{},"attachments":{},"usages":{},"services":{},"plans":{},"part_notes":{},"shops":{},"users":{}}}"#
    );
}

/// `GET /api/user` with the `X-Active-Shop` header rides the same two
/// messages, with the header's shop on both acting sessions (ADR-0009): the
/// read is shop-scoped by the per-request context, not the cookie.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_shop_header_scopes_the_read_session() {
    let (store, mut mock) = mock_ctx();
    mock.expect_hydrate_gear()
        .times(1)
        .withf(|session| session.shop() == Some(ShopId::from(1)))
        .returning(|_session| Ok(Summary::default()));
    mock.expect_read()
        .times(1)
        .withf(|session, _read| session.shop() == Some(ShopId::from(1)))
        .returning(user_state_reply());
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_headers(
        app,
        Method::GET,
        "/api/user",
        Some(&cookie),
        &[("X-Active-Shop", "1")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// `GET /api/user` without the `X-Active-Shop` header rides the same two
/// messages with no shop on the acting sessions: the full, unfiltered read.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_without_active_shop_header_has_no_shop() {
    let (store, mut mock) = mock_ctx();
    mock.expect_hydrate_gear()
        .times(1)
        .withf(|session| session.shop().is_none())
        .returning(|_session| Ok(Summary::default()));
    mock.expect_read()
        .times(1)
        .withf(|session, _read| session.shop().is_none())
        .returning(user_state_reply());
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::GET, "/api/user", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
}

/// The active shop is per-request, not per-browser (ADR-0009): a request
/// carrying the `X-Active-Shop` header does not change a subsequent
/// request's shop — the server is stateless with respect to it, so the
/// cross-tab leak is gone.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn active_shop_header_does_not_leak_across_requests() {
    let (store, mut mock) = mock_ctx();
    mock.expect_hydrate_gear()
        .times(2)
        .returning(|_session| Ok(Summary::default()));
    // Ordered expectations: the first read (the headered request) must be
    // shop-scoped, the second (headerless) must not — the first request
    // persisted nothing.
    mock.expect_read()
        .times(1)
        .withf(|session, _read| session.shop() == Some(ShopId::from(1)))
        .returning(user_state_reply());
    mock.expect_read()
        .times(1)
        .withf(|session, _read| session.shop().is_none())
        .returning(user_state_reply());
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_headers(
        app.clone(),
        Method::GET,
        "/api/user",
        Some(&cookie),
        &[("X-Active-Shop", "1")],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _headers, _body) = run(app, Method::GET, "/api/user", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
}

/// `GET /api/user/ping` is the content-free liveness probe (spec #572):
/// `204` for an authenticated session, no body, no DB work.
#[tokio::test]
async fn user_ping_returns_204() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/user/ping", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(body.is_empty());
}

/// `GET /api/user/all` (admin) rides the admin user's own channel as the
/// exec-level `AllStats` read (spec #558 §3.3) and answers `200` with the
/// per-user stats — empty for the mock's canned reply.
#[tokio::test]
async fn user_all_returns_200_and_stats() {
    let (store, mut mock) = mock_ctx();
    mock.expect_all_stats()
        .times(1)
        .returning(|_session| Ok(vec![]));
    let cookie = admin_cookie(&store).await;
    let app = build_app(&store, mock);
    let (status, _headers, body) = run(app, Method::GET, "/api/user/all", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(String::from_utf8_lossy(&body), "[]");
}

/// `GET /api/activ/{id}` rides an `Activity` read message and answers
/// `200` with the activity's JSON body.
#[tokio::test]
async fn get_activity_returns_200_and_activity_body() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::Activity(activity())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/activ/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"user_id":1,"what":1,"name":"Morning Ride","start":"2024-01-01T08:00:00Z","duration":3600,"time":null,"distance":20000,"climb":100,"descend":100,"energy":5000,"gear":null,"device_name":null,"external_id":null}"#
    );
}

/// `GET /api/part/{part}` rides a `Part` read message and answers `200`
/// with the part's JSON body.
#[tokio::test]
async fn get_part_returns_200_and_part_body() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::Part(part())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/part/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"owner":1,"what":1,"name":"Chain","vendor":"Shimano","model":"CN-HG62","purchase":"2024-01-01T00:00:00Z","last_used":"2024-01-01T00:00:00Z","disposed_at":null,"usage":"00000000-0000-0000-0000-000000000000","source":null,"shop":null}"#
    );
}

/// `GET /api/part/categories` rides a `PartCategories` read message and
/// answers `200` with the JSON array of the part types the user's parts
/// use.
#[tokio::test]
async fn get_part_categories_returns_200_and_categories() {
    let (store, mut mock) = mock_ctx();
    let mut categories = std::collections::HashSet::new();
    categories.insert(PartTypeId::from(1));
    mock.expect_read()
        .times(1)
        .returning(move |_session, _read| {
            Ok(ReadOutcome::PartCategories(std::mem::take(&mut categories)))
        });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) =
        run(app, Method::GET, "/api/part/categories", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(String::from_utf8_lossy(&body), "[1]");
}

/// `GET /api/part/{part}/notes` rides a `PartNotes` read message and
/// answers `200` with the part's notes.
#[tokio::test]
async fn get_part_notes_returns_200_and_notes() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::PartNotes(vec![note()])));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/part/1/notes", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"[{"id":1,"part":1,"name":"worn","mime":null,"filename":null,"size":null,"created":"2024-01-01T00:00:00Z"}]"#
    );
}

/// `GET /api/part/notes/{id}/file` rides a `PartNoteFile` read message and
/// answers `200` with the file bytes and the note's download headers — the
/// headers/encoding stay in axum (spec #558 §3.4), so the body is the exact
/// canned bytes.
#[tokio::test]
async fn get_note_file_returns_200_file_body_and_headers() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::PartNoteFile(file_note(), b"PDFDATA".to_vec()))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, headers, body) =
        run(app, Method::GET, "/api/part/notes/1/file", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"PDFDATA");
    assert_eq!(
        headers.get(header::CONTENT_TYPE).unwrap(),
        "application/pdf"
    );
    assert_eq!(
        headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(
        headers.get(header::CONTENT_DISPOSITION).unwrap(),
        "attachment; filename=\"chain.pdf\"; filename*=UTF-8''chain.pdf"
    );
}

/// `GET /api/shop` rides a `Shops` read message and answers `200` with the
/// user's shops.
#[tokio::test]
async fn get_shops_returns_200_and_shops() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::Shops(vec![shop()])));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/shop", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"[{"id":1,"owner":1,"name":"Bike Barn","description":null,"auto_approve":false,"created_at":"2024-01-01T00:00:00Z"}]"#
    );
}

/// `GET /api/shop/{shop}` rides a `Shop` read message and answers `200`
/// with the shop's JSON body.
#[tokio::test]
async fn get_shop_returns_200_and_shop() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::Shop(shop())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/shop/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"owner":1,"name":"Bike Barn","description":null,"auto_approve":false,"created_at":"2024-01-01T00:00:00Z"}"#
    );
}

/// `GET /api/shop/{shop}/parts` rides a `ShopParts` read message and
/// answers `200` with the shop's registered parts.
#[tokio::test]
async fn get_shop_parts_returns_200_and_parts() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::ShopParts(vec![part()])));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/shop/1/parts", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let parts: Vec<Part> = serde_json::from_str(&String::from_utf8_lossy(&body)).unwrap();
    assert_eq!(parts, vec![part()]);
}

/// `GET /api/shop/search` rides a `ShopSearch` read message (the `?q=`
/// query) and answers `200` with the two-tuple body: the matched shops,
/// then their owners' public user records.
#[tokio::test]
async fn get_shop_search_returns_200_and_search() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::ShopSearch {
            shops: vec![shop()],
            users: vec![UserPublic {
                id: UserId::from(1),
                name: "Test User".to_string(),
                firstname: "Test".to_string(),
                avatar: None,
            }],
        })
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) =
        run(app, Method::GET, "/api/shop/search?q=Bike", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"[[{"id":1,"owner":1,"name":"Bike Barn","description":null,"auto_approve":false,"created_at":"2024-01-01T00:00:00Z"}],[{"id":1,"name":"Test User","firstname":"Test","avatar":null}]]"#
    );
}

/// `GET /api/shop/subscriptions` (mine) rides a `ShopSubscriptions(None)`
/// read message and answers `200` with the user's subscriptions, each
/// carrying its shop details.
#[tokio::test]
async fn get_my_subscriptions_returns_200_and_subs() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::ShopSubscriptions(vec![
            subscription_with_details(),
        ]))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) =
        run(app, Method::GET, "/api/shop/subscriptions", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let value: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&body)).unwrap();
    let subs = value.as_array().unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0]["shop_id"], serde_json::json!(1));
    assert_eq!(subs[0]["user_id"], serde_json::json!(1));
    assert_eq!(subs[0]["status"], serde_json::json!("pending"));
    assert_eq!(subs[0]["message"], serde_json::json!("hi"));
    assert_eq!(subs[0]["shop"]["name"], serde_json::json!("Bike Barn"));
}

/// `GET /api/shop/{shop}/subscriptions` rides a
/// `ShopSubscriptions(Some(shop))` read message (the shop's pending, the
/// owner's view) and answers `200` with them.
#[tokio::test]
async fn get_shop_subscriptions_returns_200_and_subs() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::ShopSubscriptions(vec![
            subscription_with_details(),
        ]))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) =
        run(app, Method::GET, "/api/shop/1/subscriptions", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    let value: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&body)).unwrap();
    let subs = value.as_array().unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0]["status"], serde_json::json!("pending"));
    assert_eq!(subs[0]["shop"]["name"], serde_json::json!("Bike Barn"));
}

/// `GET /api/shop/subscriptions/{id}` rides a `ShopSubscription` read
/// message and answers `200` with the subscription's JSON body.
#[tokio::test]
async fn get_subscription_returns_200_and_subscription() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::ShopSubscription(subscription())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) =
        run(app, Method::GET, "/api/shop/subscriptions/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"shop_id":1,"user_id":1,"status":"pending","message":"hi","response_message":null,"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#
    );
}

// ─── Write routes (canned `WriteOutcome` in, status + body) ───────────────

fn part_body(name: &str) -> String {
    format!(
        "{{\"what\":1,\"name\":\"{name}\",\"vendor\":\"V\",\"model\":\"M\",\"purchase\":\"2024-01-01T00:00:00Z\"}}"
    )
}

/// `POST /api/part` enqueues the `PartCreate` write and answers `201` with
/// the created part (extracted from the write's `Summary`).
#[tokio::test]
async fn create_part_returns_201_and_entity() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(summary_with_part())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run_json(
        app,
        Method::POST,
        "/api/part",
        Some(&cookie),
        &part_body("Chain"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"owner":1,"what":1,"name":"Chain","vendor":"Shimano","model":"CN-HG62","purchase":"2024-01-01T00:00:00Z","last_used":"2024-01-01T00:00:00Z","disposed_at":null,"usage":"00000000-0000-0000-0000-000000000000","source":null,"shop":null}"#
    );
}

/// `PUT /api/part/{part}` enqueues the `PartChange` write and answers
/// `204 No Content`.
#[tokio::test]
async fn update_part_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/part/1",
        Some(&cookie),
        &part_body("Drainage"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// A write carrying the `X-Active-Shop` header enqueues its `ApiWrite` on
/// the shop-scoped acting session (ADR-0009): the executor routes the
/// write's frame to the shop stream from that session's shop.
#[tokio::test]
async fn write_with_active_shop_header_rides_the_shop_session() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .withf(|session, _write| session.shop() == Some(ShopId::from(1)))
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_headers(
        app,
        Method::PUT,
        "/api/part/1",
        Some(&cookie),
        &[("X-Active-Shop", "1")],
        Some(&part_body("Drainage")),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// A write without the `X-Active-Shop` header enqueues its `ApiWrite` on
/// the shop-less acting session: the executor routes its frame to the main
/// stream only.
#[tokio::test]
async fn write_without_active_shop_header_rides_the_personal_session() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .withf(|session, _write| session.shop().is_none())
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/part/1",
        Some(&cookie),
        &part_body("Drainage"),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/part/{part}` enqueues the `PartDelete` write and answers
/// `204`.
#[tokio::test]
async fn delete_part_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::DELETE, "/api/part/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/part/{part}/notes` enqueues the `PartNoteCreateText` write
/// and answers `201` with the created note.
#[tokio::test]
async fn create_text_note_returns_201_and_entity() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(summary_with_note())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run_json(
        app,
        Method::POST,
        "/api/part/1/notes",
        Some(&cookie),
        r#"{"name":"worn"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"part":1,"name":"worn","mime":null,"filename":null,"size":null,"created":"2024-01-01T00:00:00Z"}"#
    );
}

/// `POST /api/part/{part}/notes/file` parses the multipart upload and
/// enqueues the `PartNoteCreateFile` write (the file's args recorded) and
/// answers `201` with the created note.
#[tokio::test]
async fn create_file_note_returns_201_and_entity() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write().times(1).returning(|_session, _write| {
        Ok(WriteOutcome::Summary(Summary {
            part_notes: {
                let mut m = tb_domain::IdKeyedMap::default();
                m.upsert(file_note());
                m
            },
            ..Summary::default()
        }))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let multipart = concat!(
        "--tbnote\r\n",
        "Content-Disposition: form-data; name=\"name\"\r\n\r\n",
        "worn\r\n",
        "--tbnote\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"chain.pdf\"\r\n",
        "Content-Type: application/pdf\r\n\r\n",
        "PDFDATA\r\n",
        "--tbnote--\r\n",
    );
    let (status, _headers, body) = run_raw(
        app,
        Method::POST,
        "/api/part/1/notes/file",
        Some(&cookie),
        multipart.as_bytes(),
        "multipart/form-data; boundary=tbnote",
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(
        String::from_utf8_lossy(&body).contains("\"name\":\"worn\""),
        "body: {body:?}"
    );
}

/// `PUT /api/part/notes/{id}` enqueues the `PartNoteUpdateText` write and
/// answers `204`.
#[tokio::test]
async fn update_text_note_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/part/notes/1",
        Some(&cookie),
        r#"{"name":"renamed"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `PUT /api/part/notes/{id}/file` parses the multipart body and enqueues
/// the `PartNoteUpdateFile` write and answers `204`.
#[tokio::test]
async fn update_file_note_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let multipart = concat!(
        "--tbnote\r\n",
        "Content-Disposition: form-data; name=\"file\"; filename=\"chain.pdf\"\r\n",
        "Content-Type: application/pdf\r\n\r\n",
        "PDFDATA\r\n",
        "--tbnote--\r\n",
    );
    let (status, _headers, _body) = run_raw(
        app,
        Method::PUT,
        "/api/part/notes/1/file",
        Some(&cookie),
        multipart.as_bytes(),
        "multipart/form-data; boundary=tbnote",
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/part/notes/{id}/file` enqueues the `PartNoteRemoveFile`
/// write and answers `204`.
#[tokio::test]
async fn remove_file_note_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) =
        run(app, Method::DELETE, "/api/part/notes/1/file", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/part/notes/{id}` enqueues the `PartNoteDelete` write and
/// answers `204`.
#[tokio::test]
async fn delete_note_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) =
        run(app, Method::DELETE, "/api/part/notes/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/shop` enqueues the `ShopCreate` write and answers `201` with
/// the created shop.
#[tokio::test]
async fn create_shop_returns_201_and_entity() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(summary_with_shop())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run_json(
        app,
        Method::POST,
        "/api/shop",
        Some(&cookie),
        r#"{"name":"Bike Barn","description":null,"auto_approve":false}"#,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"owner":1,"name":"Bike Barn","description":null,"auto_approve":false,"created_at":"2024-01-01T00:00:00Z"}"#
    );
}

/// `PUT /api/shop/{shop}` enqueues the `ShopUpdate` write and answers
/// `204`.
#[tokio::test]
async fn update_shop_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/shop/1",
        Some(&cookie),
        r#"{"name":"Renamed","description":null,"auto_approve":true}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/shop/{shop}` enqueues the `ShopDelete` write and answers
/// `204`.
#[tokio::test]
async fn delete_shop_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::DELETE, "/api/shop/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/shop/{shop}/parts` enqueues the `ShopRegisterPart` write and
/// answers `204`.
#[tokio::test]
async fn register_shop_part_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/shop/1/parts",
        Some(&cookie),
        r#"{"part_id":1}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/shop/{shop}/parts/{part}` enqueues the
/// `ShopUnregisterPart` write and answers `204`.
#[tokio::test]
async fn unregister_shop_part_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) =
        run(app, Method::DELETE, "/api/shop/1/parts/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/shop/subscriptions` enqueues the `ShopSubscriptionCreate`
/// write, then reads the user's subscriptions back (a `ShopSubscription`
/// is in no `Summary`, spec §6.2) and answers `201` with the bare
/// subscription projected from the with-details composite.
#[tokio::test]
async fn create_subscription_returns_201_and_entity() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    mock.expect_read().times(1).returning(|_session, _read| {
        Ok(ReadOutcome::ShopSubscriptions(vec![
            subscription_with_details(),
        ]))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run_json(
        app,
        Method::POST,
        "/api/shop/subscriptions",
        Some(&cookie),
        r#"{"shop_id":1,"message":"hi"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"shop_id":1,"user_id":1,"status":"pending","message":"hi","response_message":null,"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#
    );
}

/// `POST /api/shop/subscriptions/{id}/approve` enqueues the
/// `ShopSubscriptionApprove` write and answers `204`.
#[tokio::test]
async fn approve_subscription_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/shop/subscriptions/1/approve",
        Some(&cookie),
        r#"{"message":"welcome"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/shop/subscriptions/{id}/reject` enqueues the
/// `ShopSubscriptionReject` write and answers `204`.
#[tokio::test]
async fn reject_subscription_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/shop/subscriptions/1/reject",
        Some(&cookie),
        r#"{"message":"nope"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/shop/subscriptions/{id}` enqueues the
/// `ShopSubscriptionCancel` write and answers `204`.
#[tokio::test]
async fn cancel_subscription_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(
        app,
        Method::DELETE,
        "/api/shop/subscriptions/1",
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/activ/{id}` enqueues the `ActivityUpdate` write (the path
/// id and the body's id must agree) and answers `204`.
#[tokio::test]
async fn update_activity_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/activ/1",
        Some(&cookie),
        r#"{"id":1,"user_id":1,"what":1,"name":"Morning Ride","start":"2024-01-01T08:00:00Z","duration":3600,"time":null,"distance":20000,"climb":100,"descend":100,"energy":5000,"gear":null,"device_name":null,"external_id":null}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `PUT /api/activ/{id}` answers `400` when the path id differs from the
/// body's id: the request-internal consistency check is the presentation's,
/// and it fires before any write is enqueued (the op has no path-id concept).
#[tokio::test]
async fn update_activity_id_mismatch_returns_400() {
    let (store, mock) = mock_ctx();
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    // Path id 1, body id 2: the mismatch is the presentation's guard.
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/activ/1",
        Some(&cookie),
        r#"{"id":2,"user_id":1,"what":1,"name":"Morning Ride","start":"2024-01-01T08:00:00Z","duration":3600,"time":null,"distance":20000,"climb":100,"descend":100,"energy":5000,"gear":null,"device_name":null,"external_id":null}"#,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// `DELETE /api/activ/{id}` enqueues the `ActivityDelete` write and
/// answers `204`.
#[tokio::test]
async fn delete_activity_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::DELETE, "/api/activ/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/activ/defaultgear` enqueues the `ActivityDefaultGear` write
/// and answers `204`.
#[tokio::test]
async fn default_gear_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/activ/defaultgear",
        Some(&cookie),
        r#"1"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/activ/descend` enqueues the `ActivityDescend` write (the raw
/// CSV body) and answers `200` with the match report `{good, bad}` (the
/// spec §6.2 deviation recorded on issue #446): the report is an operation
/// result, not `Summary` state — the matched activity's updated state
/// arrives as the executor's stream frame (the seam suite's territory).
#[tokio::test]
async fn descend_answers_with_match_report() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write().times(1).returning(|_session, _write| {
        Ok(WriteOutcome::Descend {
            summary: Summary::default(),
            report: DescendReport {
                good: vec!["Morning Ride at 2023-05-18 22:13:20".to_string()],
                bad: vec!["Phantom Ride at 2023-06-01 08:00:00".to_string()],
            },
        })
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let csv = "Date,Title,Total Descent\n2023-05-18 22:13:20,Morning Ride,900\n2023-06-01 08:00:00,Phantom Ride,500\n";
    let (status, _headers, body) =
        run_json(app, Method::POST, "/api/activ/descend", Some(&cookie), csv).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"good":["Morning Ride at 2023-05-18 22:13:20"],"bad":["Phantom Ride at 2023-06-01 08:00:00"]}"#
    );
}

/// `POST /api/part/attach` enqueues the `AttachmentAttach` write and
/// answers `204`.
#[tokio::test]
async fn attach_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/part/attach",
        Some(&cookie),
        r#"{"part_id":2,"time":"2024-01-01T00:00:00Z","gear":1,"hook":3,"all":false}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/part/detach` enqueues the `AttachmentDetach` write and
/// answers `204`.
#[tokio::test]
async fn detach_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/part/detach",
        Some(&cookie),
        r#"{"part_id":2,"time":"2024-01-01T00:00:00Z","gear":1,"hook":3,"all":false}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/part/dispose` enqueues the `AttachmentDispose` write and
/// answers `204`.
#[tokio::test]
async fn dispose_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/part/dispose",
        Some(&cookie),
        r#"{"part_id":2,"time":"2024-01-01T00:00:00Z","all":false}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/part/recover` enqueues the `AttachmentRecover` write and
/// answers `204`.
#[tokio::test]
async fn recover_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/part/recover",
        Some(&cookie),
        r#"{"part_id":2,"time":"2024-01-01T00:00:00Z","all":false}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/service` enqueues the `ServiceCreate` write and answers
/// `201` with the created service.
#[tokio::test]
async fn create_service_returns_201_and_entity() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(summary_with_service())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run_json(
        app,
        Method::POST,
        "/api/service",
        Some(&cookie),
        r#"{"part_id":1,"time":"2024-01-01T00:00:00Z","name":"Drainage","notes":"ok","plans":[]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":"00000000-0000-0000-0000-000000000000","part_id":1,"time":"2024-01-01T00:00:00Z","redone":"2024-01-01T00:00:00Z","name":"Drainage","notes":"ok","usage":"00000000-0000-0000-0000-000000000000","successor":null,"plans":[]}"#
    );
}

/// `PUT /api/service` enqueues the `ServiceUpdate` write and answers
/// `204`.
#[tokio::test]
async fn update_service_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/service",
        Some(&cookie),
        r#"{"id":"00000000-0000-0000-0000-000000000000","part_id":1,"time":"2024-01-01T00:00:00Z","redone":"2024-01-01T00:00:00Z","name":"Drainage","notes":"ok","usage":"00000000-0000-0000-0000-000000000000","successor":null,"plans":[]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/service/{id}` enqueues the `ServiceDelete` write and
/// answers `204`.
#[tokio::test]
async fn delete_service_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(
        app,
        Method::DELETE,
        &format!("/api/service/{NIL}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/service/redo` enqueues the `ServiceRedo` write and answers
/// `204`.
#[tokio::test]
async fn redo_service_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/service/redo",
        Some(&cookie),
        r#"{"id":"00000000-0000-0000-0000-000000000000","part_id":1,"time":"2024-01-01T00:00:00Z","redone":"2024-01-01T00:00:00Z","name":"Drainage","notes":"ok","usage":"00000000-0000-0000-0000-000000000000","successor":null,"plans":[]}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `POST /api/plan` enqueues the `ServicePlanCreate` write and answers
/// `201` with the created plan.
#[tokio::test]
async fn create_plan_returns_201_and_entity() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(summary_with_plan())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run_json(
        app,
        Method::POST,
        "/api/plan",
        Some(&cookie),
        r#"{"id":"00000000-0000-0000-0000-000000000000","part":1,"what":1,"hook":null,"name":"Drainage","days":null,"hours":null,"km":5000,"climb":null,"descend":null,"rides":30,"uid":null,"kJ":null}"#,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":"00000000-0000-0000-0000-000000000000","part":1,"what":1,"hook":null,"name":"Drainage","days":null,"hours":null,"km":5000,"climb":null,"descend":null,"rides":30,"uid":null,"kJ":null}"#
    );
}

/// `PUT /api/plan` enqueues the `ServicePlanUpdate` write and answers
/// `204`.
#[tokio::test]
async fn update_plan_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::PUT,
        "/api/plan",
        Some(&cookie),
        r#"{"id":"00000000-0000-0000-0000-000000000000","part":1,"what":1,"hook":null,"name":"Drainage","days":null,"hours":null,"km":5000,"climb":null,"descend":null,"rides":30,"uid":null,"kJ":null}"#,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `DELETE /api/plan/{id}` enqueues the `ServicePlanDelete` write and
/// answers `204`.
#[tokio::test]
async fn delete_plan_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(
        app,
        Method::DELETE,
        &format!("/api/plan/{NIL}"),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ─── The direct ops (the carve-out surface, spec #558 §5.2) ───────────────

/// `POST /strava/onboarding/sync?time=N` enqueues the
/// `UserOnboardingSync` write, then reads the `User` record back (the light
/// record-only read) and answers `200` with the user.
#[tokio::test]
async fn onboarding_sync_returns_200_and_user() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::UserRecord(user())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(
        app,
        Method::POST,
        "/strava/onboarding/sync?time=1700000000",
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"name":"Test User","firstname":"Test","avatar":null,"is_admin":false,"onboarding_status":"pending"}"#
    );
}

/// `POST /strava/onboarding/postpone` enqueues the
/// `UserOnboardingPostpone` write, then reads the `User` record back
/// (the light record-only read) and answers `200` with the user.
#[tokio::test]
async fn onboarding_postpone_returns_200_and_user() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Ok(WriteOutcome::Summary(Summary::default())));
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Ok(ReadOutcome::UserRecord(user())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(
        app,
        Method::POST,
        "/strava/onboarding/postpone",
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        String::from_utf8_lossy(&body),
        r#"{"id":1,"name":"Test User","firstname":"Test","avatar":null,"is_admin":false,"onboarding_status":"pending"}"#
    );
}

/// `GET /api/activ/rescan` (admin) is the rescan direct op (the spec
/// §3/§6.4 carve-out) and answers `204`. The op's `stop_all` (the open
/// streams end, the clients re-hydrate) is the seam suite's territory.
#[tokio::test]
async fn rescan_answers_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_rescan().times(1).returning(|_session| Ok(()));
    let app = build_app(&store, mock);
    let admin = admin_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::GET, "/api/activ/rescan", Some(&admin)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `GET /strava/sync/{id}` (admin) is the per-user sync direct op (with the
/// route's `?time=`) and answers `204`. The op's enqueue/wake/await
/// mechanism is the seam suite's territory.
#[tokio::test]
async fn admin_sync_returns_204() {
    let (store, mut mock) = mock_ctx();
    mock.expect_sync()
        .times(1)
        .returning(|_session, _uid, _time| Ok(()));
    let app = build_app(&store, mock);
    let admin = admin_cookie(&store).await;
    let (status, _headers, _body) = run(
        app,
        Method::GET,
        "/strava/sync/1?time=1700000000",
        Some(&admin),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// `GET /strava/sync?time=0` (admin) is the trigger-processing direct op and
/// answers `200`; the `?user_id=` query narrows it.
#[tokio::test]
async fn admin_trigger_processing_returns_200() {
    let (store, mut mock) = mock_ctx();
    mock.expect_trigger_processing()
        .times(1)
        .withf(|_session, user, time, _migrate| user.is_none() && *time == 0)
        .returning(|_session, _u, _t, _m| Ok(()));
    mock.expect_trigger_processing()
        .times(1)
        .withf(|_session, user, time, _migrate| user == &Some(UserId::from(1)) && *time == 0)
        .returning(|_session, _u, _t, _m| Ok(()));
    let app = build_app(&store, mock);
    let admin = admin_cookie(&store).await;
    let (status, _headers, body) = run(
        app.clone(),
        Method::GET,
        "/strava/sync?time=0",
        Some(&admin),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(String::from_utf8_lossy(&body), "null");
    let (status, _headers, _body) = run(
        app,
        Method::GET,
        "/strava/sync?time=0&user_id=1",
        Some(&admin),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

/// `POST /strava/callback` (the webhook) is the accept direct op: the
/// event's wire JSON is recorded, and the answer is `200`. The op's queue
/// insert + wake is the seam suite's territory.
#[tokio::test]
async fn webhook_accepts_the_event() {
    let (store, mut mock) = mock_ctx();
    mock.expect_accept_event()
        .times(1)
        .returning(|_event| Ok(()));
    let app = build_app(&store, mock);
    let (status, _headers, body) = run_json(
        app,
        Method::POST,
        "/strava/callback",
        None,
        r#"{"object_type":"activity","object_id":7,"aspect_type":"create","updates":{},"owner_id":1,"subscription_id":0,"event_time":1700000000}"#,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(String::from_utf8_lossy(&body), "null");
}

/// A failed webhook ingest 500s to Strava (its own retry policy applies) —
/// the mock's `Err` direct, no failing `begin` anywhere.
#[tokio::test]
async fn webhook_ingest_failure_500s() {
    let (store, mut mock) = mock_ctx();
    mock.expect_accept_event().times(1).returning(|_event| {
        Err(tb_domain::Error::AnyFailure(anyhow::anyhow!(
            "ingest failed"
        )))
    });
    let app = build_app(&store, mock);
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/strava/callback",
        None,
        r#"{"object_type":"activity","object_id":7,"aspect_type":"create","updates":{},"owner_id":1,"subscription_id":0,"event_time":1700000000}"#,
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

/// `GET /strava/bikes/{id}` is the gear redirect direct op: the Strava URL
/// permanent-redirects, and a failed op goes home (`/`).
#[tokio::test]
async fn redirect_gear_permains_to_strava_url() {
    let (store, mut mock) = mock_ctx();
    mock.expect_gear_url()
        .times(1)
        .withf(|_session, id| *id == PartId::from(7))
        .returning(|_session, _id| Ok("https://www.strava.com/bikes/7".to_string()));
    mock.expect_gear_url()
        .times(1)
        .withf(|_session, id| *id == PartId::from(9))
        .returning(|_session, _id| Err(tb_domain::Error::NotFound("part 9".to_string())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, headers, _body) =
        run(app.clone(), Method::GET, "/strava/bikes/7", Some(&cookie)).await;
    assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(headers["location"], "https://www.strava.com/bikes/7");
    let (status, headers, _body) = run(app, Method::GET, "/strava/bikes/9", Some(&cookie)).await;
    assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(headers["location"], "/");
}

/// `GET /strava/activities/{id}` is the activity redirect direct op: the
/// Strava URL permanent-redirects.
#[tokio::test]
async fn redirect_activity_permains_to_strava_url() {
    let (store, mut mock) = mock_ctx();
    mock.expect_activity_url()
        .times(1)
        .returning(|_session, _id| Ok("https://www.strava.com/activities/8".to_string()));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, headers, _body) =
        run(app, Method::GET, "/strava/activities/8", Some(&cookie)).await;
    assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(headers["location"], "https://www.strava.com/activities/8");
}

/// `GET /strava/users/{id}` is the user redirect direct op (no user
/// context — the route is public): the Strava URL permanent-redirects.
#[tokio::test]
async fn redirect_user_permains_to_strava_url() {
    let (store, mut mock) = mock_ctx();
    mock.expect_user_url()
        .times(1)
        .returning(|_id| Ok("https://www.strava.com/athletes/3".to_string()));
    let app = build_app(&store, mock);
    let (status, headers, _body) = run(app, Method::GET, "/strava/users/3", None).await;
    assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    assert_eq!(headers["location"], "https://www.strava.com/athletes/3");
}

/// `POST /strava/disable/{id}` (admin) is the deauthorization direct op and
/// answers `200`.
#[tokio::test]
async fn deauthorize_answers_200() {
    let (store, mut mock) = mock_ctx();
    mock.expect_deauthorize()
        .times(1)
        .returning(|_session, _id| Ok(()));
    let app = build_app(&store, mock);
    let admin = admin_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::POST, "/strava/disable/1", Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(String::from_utf8_lossy(&body), "null");
}

/// `DELETE /strava/delete/{id}` (admin) is the user delete direct op and
/// answers `200`.
#[tokio::test]
async fn delete_user_answers_200() {
    let (store, mut mock) = mock_ctx();
    mock.expect_delete_user()
        .times(1)
        .returning(|_session, _id| Ok(()));
    let app = build_app(&store, mock);
    let admin = admin_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::DELETE, "/strava/delete/1", Some(&admin)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(String::from_utf8_lossy(&body), "null");
}

// ─── The error mapping (the mock's `Err`, no failing `begin`) ─────────────

/// A read's `NotFound` maps to `404` (the mock's `Err` direct).
#[tokio::test]
async fn read_not_found_maps_to_404() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Err(tb_domain::Error::NotFound("part 1".to_string())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, body) = run(app, Method::GET, "/api/part/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, "Object not found: part 1".as_bytes());
}

/// A read's `DatabaseFailure` maps to `500`.
#[tokio::test]
async fn read_database_failure_maps_to_500() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read().times(1).returning(|_session, _read| {
        Err(tb_domain::Error::DatabaseFailure(anyhow::anyhow!(
            "db down"
        )))
    });
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::GET, "/api/part/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

/// A read's `Forbidden` maps to `403`.
#[tokio::test]
async fn read_forbidden_maps_to_403() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Err(tb_domain::Error::Forbidden("not yours".to_string())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::GET, "/api/shop/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

/// A read's `TryAgain` maps to `429`.
#[tokio::test]
async fn read_try_again_maps_to_429() {
    let (store, mut mock) = mock_ctx();
    mock.expect_read()
        .times(1)
        .returning(|_session, _read| Err(tb_domain::Error::TryAgain("rate limited")));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run(app, Method::GET, "/api/part/1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

/// A write's `BadRequest` maps to `400`.
#[tokio::test]
async fn write_bad_request_maps_to_400() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Err(tb_domain::Error::BadRequest("malformed".to_string())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/part",
        Some(&cookie),
        &part_body("Chain"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// A write's `Conflict` maps to `409`.
#[tokio::test]
async fn write_conflict_maps_to_409() {
    let (store, mut mock) = mock_ctx();
    mock.expect_write()
        .times(1)
        .returning(|_session, _write| Err(tb_domain::Error::Conflict("in use".to_string())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_json(
        app,
        Method::POST,
        "/api/part",
        Some(&cookie),
        &part_body("Chain"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

// ─── The SSE contract (the mock's real broadcast/watch pairs) ─────────────

/// Set up the mock's `expect_subscribe` with real broadcast/watch channels.
/// Returns the frame sender (push `Summary` frames) and the fate sender
/// (flip `ExecutorStatus`), plus the mock ready for [`build_app`].
fn mock_sse(
    mock: &mut MockServiceApi,
) -> (
    tokio::sync::broadcast::Sender<Summary>,
    tokio::sync::watch::Sender<ExecutorStatus>,
) {
    let (frame_tx, _) = tokio::sync::broadcast::channel(16);
    let (fate_tx, fate_rx) = tokio::sync::watch::channel(ExecutorStatus::Running);
    let frame_tx_inner = frame_tx.clone();
    mock.expect_subscribe().times(1..).returning(move |_uid| {
        let frame_rx = frame_tx_inner.subscribe();
        let mut frx = fate_rx.clone();
        let _ = frx.borrow_and_update();
        Ok((frame_rx, frx))
    });
    (frame_tx, fate_tx)
}

/// Set up the mock's `expect_subscribe_shop` with a real broadcast channel
/// (the shop stream is a standalone channel — no `ExecutorStatus`, spec
/// #517, ticket #583). Returns the frame sender (push `Summary` frames) and
/// the mock ready for [`build_app`].
fn mock_sse_shop(mock: &mut MockServiceApi) -> tokio::sync::broadcast::Sender<Summary> {
    let (frame_tx, _) = tokio::sync::broadcast::channel(16);
    let frame_tx_inner = frame_tx.clone();
    mock.expect_subscribe_shop()
        .times(1..)
        .returning(move |_uid, _sid| Ok(frame_tx_inner.subscribe()));
    frame_tx
}

/// A connected stream is `text/event-stream` and emits `: ping` heartbeats
/// (the SSE `KeepAlive` cadence is the 15s `tb_exec::HEARTBEAT_INTERVAL`
/// constant, spec #558 §2.1, so the window spans one full cadence).
#[tokio::test]
async fn stream_is_event_stream_and_heartbeats() {
    let (store, mut mock) = mock_ctx();
    let (_frame_tx, _fate_tx) = mock_sse(&mut mock);
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, headers, mut body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "text/event-stream",
        "the stream must be an event stream"
    );
    let frames = read_sse_frames(&mut body, 3, Duration::from_secs(18)).await;
    assert!(
        frames.iter().any(|f| f.contains(": ping")),
        "expected a `: ping` heartbeat within one `HEARTBEAT_INTERVAL` cadence, got: {frames:?}"
    );
}

/// A frame pushed on the mock's broadcast (the executor's frame channel)
/// is forwarded on the stream as a `data:` event carrying the `Summary`
/// JSON (spec §2, §4.5).
#[tokio::test]
async fn stream_forwards_summary_frames() {
    let (store, mut mock) = mock_ctx();
    let (frame_tx, _fate_tx) = mock_sse(&mut mock);
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, mut body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    // `run_sse` returns only after the endpoint registered this stream
    // (the `subscribe` ran inside the handler), so the frame below cannot
    // be missed — no sleep needed.
    frame_tx
        .send(Summary::default())
        .expect("the stream is attached");
    let frames = read_sse_frames(&mut body, 3, Duration::from_secs(2)).await;
    assert!(
        frames.iter().any(|f| f.contains(
            "data: {\"activities\":{},\"parts\":{},\"attachments\":{},\"usages\":{},\"services\":{},\"plans\":{},\"part_notes\":{},\"shops\":{},\"users\":{}}"
        )),
        "expected the Summary JSON as a `data:` frame, got: {frames:?}"
    );
}

/// A clean reap keeps the stream alive (spec §4.5, amended — #446): the
/// fate flip to `Reaped` yields a `: ping` heartbeat (the settle event),
/// the endpoint re-subscribes, and a later frame is still forwarded.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stream_survives_a_reaped_executor() {
    let (store, mut mock) = mock_ctx();
    let (frame_tx, fate_tx) = mock_sse(&mut mock);
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, mut body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    fate_tx.send_replace(ExecutorStatus::Reaped);
    // The settle heartbeat (the reap's `: ping`).
    let settled = read_sse_frames(&mut body, 1, Duration::from_secs(2)).await;
    assert!(
        settled.iter().any(|f| f.contains(": ping")),
        "a reaped executor must yield a heartbeat, got: {settled:?}"
    );
    // The frame is sent after the re-subscribe the next body poll
    // triggers (a frame sent before it would reach the old, dropped
    // receiver): the 100ms delay bounds the re-subscribe, the 3s read
    // bounds the delivery.
    let sender = frame_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        sender
            .send(Summary::default())
            .expect("the stream re-subscribed");
    });
    let frames = read_sse_frames(&mut body, 2, Duration::from_secs(10)).await;
    assert!(
        frames.iter().any(|f| f.contains("data:")),
        "a frame pushed after the reap must be forwarded, got: {frames:?}"
    );
}

/// A dead executor ends the stream (spec §4.5): the fate flip to `Dead`
/// closes the SSE body (the client's native reconnect respawns the
/// executor and re-hydrates).
#[tokio::test]
async fn stream_ends_on_a_dead_executor() {
    let (store, mut mock) = mock_ctx();
    let (_frame_tx, fate_tx) = mock_sse(&mut mock);
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, mut body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    fate_tx.send_replace(ExecutorStatus::Dead);
    assert_stream_ends(&mut body).await;
}

/// Assert an SSE body **closes** (the stream's `unfold` yields `None`):
/// bounded by a deadline, so a stream that stays open fails the test.
/// Frames that arrive before the close are drained and ignored.
async fn assert_stream_ends(body: &mut axum::body::Body) {
    use http_body_util::BodyExt;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        match tokio::time::timeout_at(deadline, body.frame()).await {
            Ok(Some(Ok(frame))) => drop(frame),
            Ok(Some(Err(err))) => panic!("the stream errored instead of closing: {err:?}"),
            Ok(None) => return,
            Err(_) => panic!("the stream did not end in time"),
        }
    }
}

/// `GET /api/user/stream?shop=X` with the shop owner's session returns a 200
/// `text/event-stream` and forwards the shop channel's frames (spec #517,
/// ticket #583). The shop stream is a standalone channel — no `ExecutorStatus`.
#[tokio::test]
async fn shop_stream_owner_forwards_frames() {
    let (store, mut mock) = mock_ctx();
    let frame_tx = mock_sse_shop(&mut mock);
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, headers, mut body) = run_sse(app, "/api/user/stream?shop=1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers[header::CONTENT_TYPE],
        "text/event-stream",
        "the shop stream must be an event stream"
    );
    // `run_sse` returns only after the endpoint opened this stream (the
    // `subscribe_shop` ran inside the handler), so the frame below cannot
    // be missed — no sleep needed.
    frame_tx
        .send(Summary::default())
        .expect("the shop stream is attached");
    let frames = read_sse_frames(&mut body, 3, Duration::from_secs(2)).await;
    assert!(
        frames.iter().any(|f| f.contains(
            "data: {\"activities\":{},\"parts\":{},\"attachments\":{},\"usages\":{},\"services\":{},\"plans\":{},\"part_notes\":{},\"shops\":{},\"users\":{}}"
        )),
        "expected the Summary JSON as a `data:` frame, got: {frames:?}"
    );
}

/// `GET /api/user/stream?shop=X` with a non-owner's session returns a 403
/// (the `subscribe_shop`'s `Forbidden` maps to 403, spec #517, ticket #583).
#[tokio::test]
async fn shop_stream_non_owner_is_forbidden() {
    let (store, mut mock) = mock_ctx();
    mock.expect_subscribe_shop()
        .times(1)
        .returning(|_uid, _sid| Err(tb_domain::Error::Forbidden("not yours".to_string())));
    let app = build_app(&store, mock);
    let cookie = user_cookie(&store).await;
    let (status, _headers, _body) = run_sse(app, "/api/user/stream?shop=1", Some(&cookie)).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
