//! Test infrastructure for the `tb_axum` crate.
//!
//! Builds the production router stack without a database: the state carries
//! the recording [`StubApi`] (the suite's `ServiceApi` adapter, spec #558
//! §8) — canned replies in, every call recorded out — and sessions are backed
//! by a `MemoryStore` that tests can seed.

#![allow(clippy::too_many_arguments)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, Once};

use axum::Router;
use http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use tb_domain::*;
use tb_exec::{ExecutorStatus, ServiceApi};
use tb_strava::{StravaId, StravaStat, StravaUser, client::StravaAthleteInfo};
use tokio::sync::{broadcast, watch};
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

// ─── StubApi: the recording service API (spec #558 §8) ───────────────────
//
// The suite's `ServiceApi` adapter (the seam's second, the `Registry`'s
// sibling): canned `ReadOutcome`/`WriteOutcome`/`TbResult` replies in,
// every call recorded out. The route contract tests assert the status code
// + exact JSON body **and** the recorded call — the exact read/write/
// direct-op with args the handler made (a capability the old live-loop
// suite lacked). Error mapping (404/500/401/403/400/409/429) is the stub
// returning `Err(...)` directly — no failing `begin` anywhere. The SSE
// contract tests get real channel pairs: `subscribe` hands out receivers
// on the per-user broadcast (the frames) + watch (the executor's fate),
// and the tests push a frame or flip the fate through [`StubApi::senders`].
// No loop, no in-memory store, no executor tokio plumbing.

/// One call a handler made on the service API (spec #558 §8): the exact
/// read/write/direct op with the handler's args — the recorded-call
/// assertions' target.
#[derive(Clone, Debug, PartialEq)]
pub enum StubCall {
    /// A per-user read (the `ApiRead` descriptor, spec #558 §3).
    Read(UserId, ApiRead),
    /// A per-user write (the `ApiWrite` descriptor).
    Write(UserId, ApiWrite),
    /// The hydration write (the `GET /user/summary` route's first message).
    HydrateGear(UserId),
    /// The stream subscription (the SSE endpoint; re-called after a reap).
    Subscribe(UserId),
    /// The admin stats read (the `GET /user/all` route).
    AllStats(UserId),
    /// The admin rescan direct op.
    Rescan,
    /// The per-user admin sync direct op (with the route's `?time=`).
    Sync(UserId, i64),
    /// The all-users admin sync direct op (`?user_id=` narrows it).
    SyncAll {
        user: Option<UserId>,
        time: i64,
        migrate: bool,
    },
    /// The login upsert direct op (the args the outbound exchange fed it;
    /// the refresh token's secret, presence-only — the wire parsing stays
    /// in axum, beside the exchange).
    LoginFromToken {
        id: StravaId,
        firstname: String,
        lastname: String,
        avatar: Option<String>,
        refresh: Option<String>,
    },
    /// The webhook accept direct op: the event as its wire JSON (`InEvent`
    /// is the wire shape; the recorded value is what the handler sent).
    AcceptEvent(serde_json::Value),
    /// The gear redirect URL direct op.
    GearUrl(UserId, PartId),
    /// The activity redirect URL direct op.
    ActivityUrl(UserId, ActivityId),
    /// The user redirect URL direct op.
    UserUrl(UserId),
    /// The deauthorization direct op.
    Deauthorize(UserId),
    /// The user delete direct op.
    DeleteUser(UserId),
}

/// The recording service API (spec #558 §8): the suite's `ServiceApi`
/// adapter. One canned reply per expected call, queued in the order the
/// handler makes them (a test that enqueues fewer replies than the handler
/// calls panics with the missing call's name — an unexpected call is a
/// contract violation, not a flake); every call is recorded for the
/// assertions. The SSE pairs are real `broadcast`/`watch` channels the
/// stub owns (one per user, created on first `subscribe`), so the wire
/// tests push frames and flip the executor's fate like the loop would.
#[derive(Default)]
pub struct StubApi {
    calls: Mutex<Vec<StubCall>>,
    read: Mutex<VecDeque<TbResult<ReadOutcome>>>,
    write: Mutex<VecDeque<TbResult<WriteOutcome>>>,
    hydrate: Mutex<VecDeque<TbResult<Summary>>>,
    all_stats: Mutex<VecDeque<TbResult<Vec<StravaStat>>>>,
    /// The direct ops that answer `()` (rescan, the syncs, the webhook
    /// accept, the deauth, the delete).
    unit: Mutex<VecDeque<TbResult<()>>>,
    /// The redirect-URL direct ops (gear, activity, user).
    url: Mutex<VecDeque<TbResult<String>>>,
    /// The SSE pairs the stub owns: one broadcast (the frames) + watch
    /// (the executor's fate) per subscribed user.
    streams: Mutex<HashMap<UserId, (broadcast::Sender<Summary>, watch::Sender<ExecutorStatus>)>>,
}

impl StubApi {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue the reply the next `read` call gets.
    pub fn reply_read(&self, reply: TbResult<ReadOutcome>) {
        self.read.lock().expect("stub").push_back(reply);
    }

    /// Queue the reply the next `write` call gets.
    pub fn reply_write(&self, reply: TbResult<WriteOutcome>) {
        self.write.lock().expect("stub").push_back(reply);
    }

    /// Queue the reply the next `hydrate_gear` call gets.
    pub fn reply_hydrate(&self, reply: TbResult<Summary>) {
        self.hydrate.lock().expect("stub").push_back(reply);
    }

    /// Queue the reply the next `all_stats` call gets.
    pub fn reply_all_stats(&self, reply: TbResult<Vec<StravaStat>>) {
        self.all_stats.lock().expect("stub").push_back(reply);
    }

    /// Queue the reply the next unit direct op (rescan, the syncs, the
    /// webhook accept, the deauth, the delete) gets.
    pub fn reply_unit(&self, reply: TbResult<()>) {
        self.unit.lock().expect("stub").push_back(reply);
    }

    /// Queue the reply the next redirect-URL direct op gets.
    pub fn reply_url(&self, reply: TbResult<String>) {
        self.url.lock().expect("stub").push_back(reply);
    }

    /// Every call the handlers made, in order — the recorded-call
    /// assertions' subject.
    pub fn calls(&self) -> Vec<StubCall> {
        self.calls.lock().expect("stub").clone()
    }

    /// The SSE senders for `user_id` (the stub owns the real
    /// broadcast/watch pair the `subscribe` call handed the endpoint):
    /// push a frame on the broadcast, flip the executor's fate on the
    /// watch. Panics if the user never subscribed — the pair does not
    /// exist yet.
    pub fn senders(
        &self,
        user_id: UserId,
    ) -> (broadcast::Sender<Summary>, watch::Sender<ExecutorStatus>) {
        self.streams
            .lock()
            .expect("stub")
            .get(&user_id)
            .cloned()
            .unwrap_or_else(|| panic!("StubApi: {user_id:?} never subscribed"))
    }
}

fn stub_reply<T>(queue: &Mutex<VecDeque<TbResult<T>>>, what: &str) -> TbResult<T> {
    queue
        .lock()
        .expect("stub")
        .pop_front()
        .unwrap_or_else(|| panic!("StubApi: no canned {what} reply queued"))
}

#[async_trait::async_trait]
impl ServiceApi for StubApi {
    async fn read(&self, user_id: UserId, read: ApiRead) -> TbResult<ReadOutcome> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::Read(user_id, read));
        stub_reply(&self.read, "read")
    }

    async fn write(&self, user_id: UserId, write: ApiWrite) -> TbResult<WriteOutcome> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::Write(user_id, write));
        stub_reply(&self.write, "write")
    }

    async fn hydrate_gear(&self, user_id: UserId) -> TbResult<Summary> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::HydrateGear(user_id));
        stub_reply(&self.hydrate, "hydrate_gear")
    }

    async fn subscribe(
        &self,
        user_id: UserId,
    ) -> TbResult<(
        broadcast::Receiver<Summary>,
        watch::Receiver<ExecutorStatus>,
    )> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::Subscribe(user_id));
        let (frames, fate) = self
            .streams
            .lock()
            .expect("stub")
            .entry(user_id)
            .or_insert_with(|| {
                let (frames, _rx) = broadcast::channel(16);
                let (fate, _rx) = watch::channel(ExecutorStatus::Running);
                (frames, fate)
            })
            .clone();
        Ok((frames.subscribe(), fate.subscribe()))
    }

    async fn all_stats(&self, user_id: UserId) -> TbResult<Vec<StravaStat>> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::AllStats(user_id));
        stub_reply(&self.all_stats, "all_stats")
    }

    async fn rescan(&self) -> TbResult<()> {
        self.calls.lock().expect("stub").push(StubCall::Rescan);
        stub_reply(&self.unit, "rescan")
    }

    async fn sync(&self, user_id: UserId, event_time: i64) -> TbResult<()> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::Sync(user_id, event_time));
        stub_reply(&self.unit, "sync")
    }

    async fn sync_all(
        &self,
        user_id: Option<UserId>,
        event_time: i64,
        migrate: bool,
    ) -> TbResult<()> {
        self.calls.lock().expect("stub").push(StubCall::SyncAll {
            user: user_id,
            time: event_time,
            migrate,
        });
        stub_reply(&self.unit, "sync_all")
    }

    async fn login_from_token(
        &self,
        athlete: StravaAthleteInfo,
        refresh: Option<oauth2::RefreshToken>,
    ) -> TbResult<(StravaUser, bool)> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::LoginFromToken {
                id: athlete.id,
                firstname: athlete.firstname,
                lastname: athlete.lastname,
                avatar: athlete.avatar,
                refresh: refresh.map(|token| token.secret().to_owned()),
            });
        // The only route that reaches the op (`GET /strava/token`) cannot
        // complete in this suite — the outbound token exchange is a real
        // `tb_strava` op over the raw HTTP client, which the canned-response
        // hook does not intercept — so the stub records and always succeeds.
        Ok((
            StravaUser {
                id: StravaId::from(1),
                tendabike_id: UserId::from(1),
                refresh_token: None,
            },
            false,
        ))
    }

    async fn accept_event(&self, event: tb_strava::event::InEvent) -> TbResult<()> {
        self.calls.lock().expect("stub").push(StubCall::AcceptEvent(
            serde_json::to_value(&event).expect("InEvent serializes"),
        ));
        stub_reply(&self.unit, "accept_event")
    }

    async fn gear_url(&self, user: UserId, id: PartId) -> TbResult<String> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::GearUrl(user, id));
        stub_reply(&self.url, "gear_url")
    }

    async fn activity_url(&self, user: UserId, id: ActivityId) -> TbResult<String> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::ActivityUrl(user, id));
        stub_reply(&self.url, "activity_url")
    }

    async fn user_url(&self, id: UserId) -> TbResult<String> {
        self.calls.lock().expect("stub").push(StubCall::UserUrl(id));
        stub_reply(&self.url, "user_url")
    }

    async fn deauthorize(&self, tbid: UserId) -> TbResult<()> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::Deauthorize(tbid));
        stub_reply(&self.unit, "deauthorize")
    }

    async fn delete_user(&self, tbid: UserId) -> TbResult<()> {
        self.calls
            .lock()
            .expect("stub")
            .push(StubCall::DeleteUser(tbid));
        stub_reply(&self.unit, "delete_user")
    }
}
