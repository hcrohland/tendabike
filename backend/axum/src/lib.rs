//! This file contains the implementation of the presentation layer of the Tendabike server using the Axum framework.
//!
//! The presentation layer is responsible for handling HTTP requests and responses, and for translating them into
//! actions that the application layer can understand. The Axum framework is used to implement the presentation layer.
//!
//! The web layer is storage-agnostic: the state and every handler are generic over a
//! [`TxnSource`], and the concrete store (the `DbPool` from `tb_sqlx`) is injected at the
//! composition root (ADR-0005, executable spec #446 §6). It knows none of the details of
//! `tb_sqlx` — but it did choose a `PostgresStore` for its sessions, so [`start`] also
//! takes the postgres connection (the `PgPool` the caller built its store from), and it
//! owns its own logging subscriber: the composition root wires the concrete crates, not
//! the crate-internal details.
//!
//! This file defines the `start` function, which is the entry point for the presentation layer. It takes a
//! `TxnSource`, the `PgPool` for the session store, a path to the directory containing static files, and a socket address to bind to.
//! It sets up the necessary components for the presentation layer, such as the router and the middleware,
//! and starts the server.
//!
//! This file also contains the definitions of various modules that implement the endpoints for the different resources
//! of the Tendabike server, such as users, parts, attachments, activities, and Strava integration.
//!

use anyhow::Context;
use axum::Router;
use sqlx::PgPool;
use std::net::SocketAddr;
use tb_domain::TbResult;
use tb_exec::TxnSource;
use tb_strava::StravaStore;
use tower_sessions::{ExpiredDeletion, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod domain;

mod strava;
use strava::{AxumAdmin, RequestSession};

mod appstate;
use appstate::*;

mod error;
use error::*;

mod stream;

#[cfg(test)]
mod test_support;

pub fn routes<S: TxnSource + Clone + 'static>(app_state: AppState<S>) -> Router
where
    S::Conn: StravaStore,
{
    Router::new()
        .nest("/api", domain::router())
        .nest("/strava", strava::router())
        .with_state(app_state)
}

pub async fn start<S: TxnSource + Clone + Send + Sync + 'static>(
    source: S,
    pool: PgPool,
    path: std::path::PathBuf,
    addr: SocketAddr,
) -> TbResult<()>
where
    S::Conn: StravaStore,
{
    // The logging subscriber lives with the web layer it logs (the
    // composition root owns process setup, not crate-internal wiring).
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = AppState::new(source);

    // The web layer chose a `PostgresStore` for its sessions, so the
    // caller hands over the postgres connection — the same pool the
    // `TxnSource` was built from.
    let session_store = PostgresStore::new(pool);

    session_store
        .migrate()
        .await
        .context("Session store migration")?;

    let deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(600)),
    );

    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(tower_sessions::Expiry::OnInactivity(time::Duration::days(
            10,
        )))
        .with_secure(false);

    let app = routes(app_state)
        .fallback_service(tower_http::services::ServeDir::new(path))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(session_layer);

    tracing::debug!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context("Binding address")?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
        .await
        .context("Main server")?;

    deletion_task.await.ok();
    Ok(())
}

async fn shutdown_signal(deletion_task_abort_handle: tokio::task::AbortHandle) {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use axum::Router;
    use http::{Method, StatusCode, header};
    use tower_sessions::MemoryStore;

    use tb_domain::{Activity, ActivityId, ActivityStore, PartStore, Summary, UserId};

    use crate::stream::{ExecutorStatus, Registry};
    use crate::test_support::{
        LiveSource, admin_cookie, read_sse_frames, run, run_json, run_sse, test_app, test_app_live,
        user_cookie,
    };

    async fn setup() -> (Router, MemoryStore) {
        let store = MemoryStore::default();
        (test_app(store.clone()), store)
    }

    async fn expect_unauth(method: Method, uri: &str) {
        let (app, _store) = setup().await;
        let (status, _headers, body) = run(app, method, uri, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body, "Please login".as_bytes());
    }

    async fn expect_hidden_from_non_admin(method: Method, uri: &str) {
        let (app, store) = setup().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, _body) = run(app, method, uri, Some(&cookie)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn types_part_returns_part_types() {
        let (app, _store) = setup().await;
        let (status, _headers, body) = run(app, Method::GET, "/api/types/part", None).await;
        assert_eq!(status, StatusCode::OK);
        let types: Vec<serde_json::Value> =
            serde_json::from_slice(&body).expect("part types are json");
        assert!(!types.is_empty());
    }

    #[tokio::test]
    async fn types_activity_returns_activity_types() {
        let (app, _store) = setup().await;
        let (status, _headers, body) = run(app, Method::GET, "/api/types/activity", None).await;
        assert_eq!(status, StatusCode::OK);
        let types: Vec<serde_json::Value> =
            serde_json::from_slice(&body).expect("activity types are json");
        assert!(!types.is_empty());
    }

    #[tokio::test]
    async fn callback_validation_echoes_challenge() {
        let (app, _store) = setup().await;
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
        let (app, _store) = setup().await;
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
        let (app, _store) = setup().await;
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
        let (app, _store) = setup().await;
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
        let (app, _store) = setup().await;
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
        let (app, _store) = setup().await;
        let (status, headers, _body) = run(app, Method::GET, "/strava/logout", None).await;
        assert_eq!(status, StatusCode::SEE_OTHER);
        assert_eq!(headers["location"], "/");
    }

    #[tokio::test]
    async fn user_requires_auth() {
        expect_unauth(Method::GET, "/api/user").await;
    }

    #[tokio::test]
    async fn part_requires_auth() {
        expect_unauth(Method::GET, "/api/part/categories").await;
    }

    #[tokio::test]
    async fn stream_requires_auth() {
        expect_unauth(Method::GET, "/api/user/stream").await;
    }

    /// A connected stream is `text/event-stream` and emits `: ping` heartbeats
    /// (spec §4.5): the short heartbeat cadence makes one arrive quickly.
    #[tokio::test]
    async fn stream_is_event_stream_and_heartbeats() {
        let store = MemoryStore::default();
        let registry = Arc::new(Registry::new(
            Duration::from_secs(60),
            Duration::from_millis(50),
        ));
        let live = LiveSource::new().await;
        let app = test_app_live(&store, live, registry);
        let cookie = user_cookie(&store).await;
        let (status, headers, mut body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers[header::CONTENT_TYPE],
            "text/event-stream",
            "the stream must be an event stream"
        );
        let frames = read_sse_frames(&mut body, 3, Duration::from_secs(2)).await;
        assert!(
            frames.iter().any(|f| f.contains(": ping")),
            "expected a `: ping` heartbeat, got: {frames:?}"
        );
    }

    /// A frame pushed to the user's executor is forwarded on the stream as a
    /// `data:` event carrying the `Summary` JSON (spec §2, §4.5).
    #[tokio::test]
    async fn stream_forwards_summary_frames() {
        let store = MemoryStore::default();
        let registry = Arc::new(Registry::new(
            Duration::from_secs(60),
            Duration::from_secs(15),
        ));
        let live = LiveSource::new().await;
        let app = test_app_live(&store, live, registry.clone());
        let cookie = user_cookie(&store).await;
        let (status, _headers, mut body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        // `run_sse` returns only after the endpoint registered this stream
        // on the executor's frame channel (the `subscribe` ran inside the
        // handler), so the frame below cannot be missed — no sleep needed.
        registry
            .send_frame(UserId::from(1), Summary::default())
            .await;
        let frames = read_sse_frames(&mut body, 3, Duration::from_secs(2)).await;
        assert!(
            frames.iter().any(|f| f.contains("data:")),
            "expected a `data:` frame, got: {frames:?}"
        );
    }

    /// When the executor reaps on idle, the stream stays alive on heartbeats and
    /// re-subscribes (respawning the executor) so a later frame is forwarded
    /// (spec §4.5 — the stream survives the executor's reap).
    #[tokio::test]
    async fn stream_survives_executor_reap() {
        let store = MemoryStore::default();
        // A short idle window so the executor reaps quickly; a short heartbeat
        // so the reaped stream's keepalive is observable.
        let registry = Arc::new(Registry::new(
            Duration::from_millis(100),
            Duration::from_millis(50),
        ));
        let live = LiveSource::new().await;
        let app = test_app_live(&store, live, registry);
        let cookie = user_cookie(&store).await;
        let (status, _headers, mut body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        // The executor reaps on idle (~100ms here) while this stream is
        // attached. The reaped stream keeps heartbeating (it does not close),
        // so frames keep arriving across the reap (spec §9.5). Three beats at
        // a 50ms interval span the 100ms reap, so at least one is post-reap.
        let frames = read_sse_frames(&mut body, 3, Duration::from_secs(2)).await;
        assert!(
            frames.iter().filter(|f| f.contains(": ping")).count() >= 2,
            "expected heartbeats across the executor reap, got: {frames:?}"
        );
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
    async fn partnote_reaches_db_layer() {
        let (app, store) = setup().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, _body) =
            run(app, Method::GET, "/api/part/1/notes", Some(&cookie)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn partnote_update_reaches_db_layer() {
        let (app, store) = setup().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, _body) = run_json(
            app,
            Method::PUT,
            "/api/part/notes/1",
            Some(&cookie),
            r#"{"name":"renamed"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn partnote_delete_reaches_db_layer() {
        let (app, store) = setup().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, _body) =
            run(app, Method::DELETE, "/api/part/notes/1", Some(&cookie)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
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
    async fn admin_reaches_db_layer() {
        let (app, store) = setup().await;
        let cookie = admin_cookie(&store).await;
        let (status, _headers, _body) = run(app, Method::GET, "/api/user/all", Some(&cookie)).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn wrong_method_not_allowed() {
        let (app, _store) = setup().await;
        let (status, _headers, _body) = run(app, Method::POST, "/api/user/summary", None).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn unknown_path_not_found() {
        let (app, _store) = setup().await;
        let (status, _headers, _body) = run(app, Method::GET, "/api/does-not-exist", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // ─── Executor-backed write handlers (spec §6.2) ─────────────────────

    fn part_body(name: &str) -> String {
        format!(
            "{{\"what\":1,\"name\":\"{name}\",\"vendor\":\"V\",\"model\":\"M\",\"purchase\":\"2024-01-01T00:00:00Z\"}}"
        )
    }

    /// A live-app fixture: shared in-memory db + session store + registry.
    async fn live_app() -> (Router, MemoryStore, Arc<Registry>, LiveSource) {
        let store = MemoryStore::default();
        let registry = Arc::new(Registry::new(
            Duration::from_secs(60),
            Duration::from_secs(15),
        ));
        let live = LiveSource::new().await;
        let app = test_app_live(&store, live.clone(), registry.clone());
        (app, store, registry, live)
    }

    /// `POST /api/part` runs on the user's executor and answers `201` with
    /// the created part (extracted from the write's `Summary`); the shared
    /// store is actually changed.
    #[tokio::test]
    async fn create_part_returns_201_and_entity() {
        let (app, store, _registry, live) = live_app().await;
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
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("Chain"), "body: {text}");
        let mut conn = live.mem().lock().unwrap().begin();
        let parts = PartStore::part_get_all_for_userid(&mut conn, &UserId::from(1))
            .await
            .unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "Chain");
    }

    /// `PUT /api/part/{part}` runs on the executor and answers `204 No
    /// Content`; the shared store is updated.
    #[tokio::test]
    async fn update_part_returns_204() {
        let (app, store, _registry, live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, body) = run_json(
            app.clone(),
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Chain"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let part_id = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&body))
            .unwrap()["id"]
            .as_i64()
            .unwrap();
        let (status, _headers, _body) = run_json(
            app,
            Method::PUT,
            &format!("/api/part/{part_id}"),
            Some(&cookie),
            &part_body("Drainage"),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let mut conn = live.mem().lock().unwrap().begin();
        let parts = PartStore::part_get_all_for_userid(&mut conn, &UserId::from(1))
            .await
            .unwrap();
        assert_eq!(parts[0].name, "Drainage");
    }

    /// `DELETE /api/part/{part}` runs on the executor and answers `204`; the
    /// part is gone from the shared store.
    #[tokio::test]
    async fn delete_part_returns_204() {
        let (app, store, _registry, live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, body) = run_json(
            app.clone(),
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Chain"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let part_id = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&body))
            .unwrap()["id"]
            .as_i64()
            .unwrap();
        let (status, _headers, _body) = run(
            app,
            Method::DELETE,
            &format!("/api/part/{part_id}"),
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let mut conn = live.mem().lock().unwrap().begin();
        let parts = PartStore::part_get_all_for_userid(&mut conn, &UserId::from(1))
            .await
            .unwrap();
        assert!(parts.is_empty());
    }

    /// `POST /api/part/{part}/notes` answers `201` with the created note.
    #[tokio::test]
    async fn create_text_note_returns_201_and_entity() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, _body) = run_json(
            app.clone(),
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Chain"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let (status, _headers, body) = run_json(
            app,
            Method::POST,
            "/api/part/1/notes",
            Some(&cookie),
            r#"{"name":"worn"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(
            String::from_utf8_lossy(&body).contains("worn"),
            "body: {body:?}"
        );
    }

    /// `POST /strava/onboarding/sync` runs the status update on the executor,
    /// queues the Strava sync event, and answers `200` with the `User`; the
    /// executor then consumes the queued sync via the webhook's wake signal
    /// (the stream is open, as in production — the user's tab is attached —
    /// so the executor is running and the wake interrupts its idle wait).
    /// The in-memory seam has no refresh token: `NotAuth` disables the user
    /// and clears the queue.
    #[tokio::test]
    async fn onboarding_sync_returns_200_and_user() {
        let (app, store, _registry, live) = live_app().await;
        let cookie = user_cookie(&store).await;
        // The user's tab is open during onboarding: the stream keeps the
        // executor running (the wake is a signal only — it never spawns).
        let (status, _headers, _body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _headers, body) =
            run(app, Method::POST, "/strava/onboarding/sync", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            String::from_utf8_lossy(&body).contains("onboarding_status"),
            "body: {body:?}"
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            if live.strava().lock().unwrap().events.is_empty()
                || tokio::time::Instant::now() >= deadline
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            live.strava().lock().unwrap().events.is_empty(),
            "the queued sync must have been consumed"
        );
    }

    /// `POST /strava/onboarding/postpone` answers `200` with the `User`.
    #[tokio::test]
    async fn onboarding_postpone_returns_200_and_user() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, body) = run(
            app,
            Method::POST,
            "/strava/onboarding/postpone",
            Some(&cookie),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            String::from_utf8_lossy(&body).contains("onboarding_status"),
            "body: {body:?}"
        );
    }

    /// `POST /api/shop/subscriptions` runs on the executor and answers `201`
    /// with the created subscription, read back after the write (a
    /// `ShopSubscription` is in no `Summary`, spec §6.2).
    #[tokio::test]
    async fn create_subscription_returns_201_and_entity() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        // A shop to subscribe to (the create guard checks the shop exists).
        let (status, _headers, body) = run_json(
            app.clone(),
            Method::POST,
            "/api/shop",
            Some(&cookie),
            r#"{"name":"Bike Barn","description":null,"auto_approve":false}"#,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let shop_id = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&body))
            .unwrap()["id"]
            .as_i64()
            .unwrap();
        let (status, _headers, body) = run_json(
            app,
            Method::POST,
            "/api/shop/subscriptions",
            Some(&cookie),
            &format!("{{\"shop_id\":{shop_id},\"message\":\"hi\"}}"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\"status\":\"pending\""), "body: {text}");
        assert!(text.contains("\"message\":\"hi\""), "body: {text}");
    }

    /// `GET /strava/sync/{id}` (admin) enqueues a sync, wakes the target
    /// user's executor, and answers `204` only after the executor consumed
    /// the event (the queue is empty by then).
    #[tokio::test]
    async fn admin_sync_returns_204_and_consumes_queue() {
        let (app, store, _registry, live) = live_app().await;
        let cookie = admin_cookie(&store).await;
        let (status, _headers, _body) =
            run(app, Method::GET, "/strava/sync/1", Some(&cookie)).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(
            live.strava().lock().unwrap().events.is_empty(),
            "the queued sync must have been consumed"
        );
    }

    /// A Strava event queued by the webhook is consumed **promptly** by a
    /// running, idle-sleeping executor: the webhook's wake signal interrupts
    /// the executor's idle wait, so the event does not wait for the idle
    /// timeout to expire (spec §4.4/§4.6 — the in-memory wake signal).
    #[tokio::test]
    async fn webhook_wake_drains_the_queue_before_the_idle_timeout() {
        let store = MemoryStore::default();
        // A long idle window: without the wake interrupting the idle sleep,
        // the queued event would sit unconsumed for the whole 60s.
        let registry = Arc::new(Registry::new(
            Duration::from_secs(60),
            Duration::from_millis(50),
        ));
        let live = LiveSource::new().await;
        let app = test_app_live(&store, live.clone(), registry.clone());
        let cookie = user_cookie(&store).await;

        // Open a stream: the executor spawns and stays running (the attached
        // stream keeps it from reaping).
        let (status, _headers, _body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        wait_for_status(&registry, UserId::from(1), ExecutorStatus::Running).await;

        // A canary event first: once it is consumed, the executor has
        // re-probed the (now empty) queue and parked in its 60s idle sleep
        // — the state the wake has to interrupt.
        let (status, _headers, _body) = run_json(
            app.clone(),
            Method::POST,
            "/strava/callback",
            None,
            r#"{"object_type":"sync","object_id":0,"aspect_type":"create","updates":{},"owner_id":1,"subscription_id":0,"event_time":1700000000}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_queue_drained(&live).await;
        // Settle: the executor is in its idle sleep by now.
        tokio::time::sleep(Duration::from_millis(200)).await;

        // The probe event: the webhook queues it and fires the wake.
        let (status, _headers, _body) = run_json(
            app,
            Method::POST,
            "/strava/callback",
            None,
            r#"{"object_type":"sync","object_id":0,"aspect_type":"create","updates":{},"owner_id":1,"subscription_id":0,"event_time":1700000001}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        // The event is consumed well under the 60s idle timeout (bounded
        // 3s wait): the wake interrupted the idle sleep, the loop re-probed
        // and drained the queue (the seam has no refresh token, so the
        // sync's `NotAuth` disable drops the queue — consumed either way).
        assert_queue_drained(&live).await;
    }

    /// Wait (bounded) until the shared in-memory Strava queue is empty: the
    /// seam's executor consumes a queued event by dropping it (the seam has
    /// no refresh token, so `NotAuth` disables the user and clears the
    /// queue) — an empty queue is consumption.
    async fn assert_queue_drained(live: &LiveSource) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            if live.strava().lock().unwrap().events.is_empty()
                || tokio::time::Instant::now() >= deadline
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            live.strava().lock().unwrap().events.is_empty(),
            "the queued event must be consumed well under the idle timeout"
        );
    }

    /// `Registry::wake` is a pure signal to a **running** executor: with no
    /// executor running (reaped), the wake spawns nothing and the queued
    /// event stays queued — the DB queue is the source of truth; the next
    /// spawn (a write or an SSE connect) drains it (spec §4.4).
    #[tokio::test]
    async fn wake_never_spawns_a_reaped_executor() {
        let store = MemoryStore::default();
        // A short idle window so the executor reaps quickly once the stream
        // is closed.
        let registry = Arc::new(Registry::new(
            Duration::from_millis(100),
            Duration::from_millis(50),
        ));
        let live = LiveSource::new().await;
        let app = test_app_live(&store, live.clone(), registry.clone());
        let cookie = user_cookie(&store).await;
        let user = UserId::from(1);

        // Spawn the executor with a stream, then close it and let it reap.
        let (status, _headers, body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        wait_for_status(&registry, user, ExecutorStatus::Running).await;
        drop(body);
        wait_for_status(&registry, user, ExecutorStatus::Reaped).await;

        // Queue an event for the user.
        live.strava()
            .lock()
            .unwrap()
            .events
            .push(tb_strava::event::Event {
                object_type: tb_strava::event::ObjectType::Sync,
                owner_id: tb_strava::StravaId::from(1),
                event_time: 1700000000,
                ..Default::default()
            });

        // The wake is a signal only: no executor is running, so it spawns
        // nothing and the event stays queued.
        registry.wake(user).await;
        assert_ne!(
            registry.status(user).await,
            Some(ExecutorStatus::Running),
            "the wake must not spawn an executor"
        );
        assert!(
            !live.strava().lock().unwrap().events.is_empty(),
            "the queued event must stay queued for the next spawn"
        );

        // A write spawns the executor, which drains the queue.
        registry
            .write(&live, user, tb_domain::ApiWrite::UserOnboardingPostpone)
            .await
            .expect("the write must succeed");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if live.strava().lock().unwrap().events.is_empty()
                || tokio::time::Instant::now() >= deadline
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(
            live.strava().lock().unwrap().events.is_empty(),
            "the respawned executor must drain the queue"
        );
    }

    /// The admin sync endpoint hides itself from non-admins (`404`).
    #[tokio::test]
    async fn admin_sync_requires_admin() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, _body) =
            run(app, Method::GET, "/strava/sync/1", Some(&cookie)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    /// `GET /strava/hooks` is gone (spec: removed with the cutover).
    #[tokio::test]
    async fn hooks_route_is_gone() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, _body) = run(app, Method::GET, "/strava/hooks", Some(&cookie)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    /// The consolidated cutover scenario (spec §8, §9): in one scenario, (a)
    /// `GET /api/user/stream` opens with a valid session, (b) a write
    /// (`POST /api/part`) rides the user's executor and answers `201`, (c) the
    /// created part arrives as a `data:` frame on that same open stream, and
    /// (d) the old poll endpoint `GET /strava/hooks` answers `404`.
    #[tokio::test]
    async fn cutover_end_to_end_write_rides_stream_and_hooks_gone() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        // (a) The stream opens with a valid session.
        let (status, headers, mut body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers[header::CONTENT_TYPE],
            "text/event-stream",
            "the stream must be an event stream"
        );
        // (d) The old poll endpoint is gone.
        let (status, _headers, _body) =
            run(app.clone(), Method::GET, "/strava/hooks", Some(&cookie)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // (b) The write rides the user's executor (its oneshot resolves only
        // after the executor processed it and pushed the frame, so the frame
        // is already on the channel by the time the 201 is back — no sleep).
        let (status, _headers, _body) = run_json(
            app,
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Chain"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        // (c) The created entity arrives as a `data:` frame on the same
        // open stream (the frame carries the write's `Summary` JSON).
        let frames = read_sse_frames(&mut body, 3, Duration::from_secs(3)).await;
        let data: Vec<&String> = frames.iter().filter(|f| f.contains("data:")).collect();
        assert!(
            data.iter().any(|f| f.contains("Chain")),
            "expected the created part in a data frame, got: {frames:?}"
        );
    }

    /// A write enqueued on the user's executor pushes its `Summary` to the
    /// user's SSE stream.
    #[tokio::test]
    async fn write_pushes_summary_frame_to_stream() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, mut body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        let (status, _headers, _body) = run_json(
            app,
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Chain"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let frames = read_sse_frames(&mut body, 3, Duration::from_secs(3)).await;
        assert!(
            frames.iter().any(|f| f.contains("data:")),
            "expected a data frame, got: {frames:?}"
        );
    }

    /// Poll the registry's executor status until it is `expected` (bounded by
    /// a deadline, so a missing flip fails the test instead of hanging).
    async fn wait_for_status(registry: &Arc<Registry>, user: UserId, expected: ExecutorStatus) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if registry.status(user).await == Some(expected) {
                return;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "the executor status never reached {expected:?}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Assert an SSE body **closes** (the stream's `unfold` yields `None`):
    /// bounded by a deadline, so a stream that stays open fails the test.
    /// Data frames that arrive before the close are drained and ignored.
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

    /// `POST /api/activ/descend` rides the user's executor and answers `200`
    /// with the match report `{good, bad}` (the spec §6.2 deviation recorded
    /// on issue #446): the report is an operation result, not `Summary`
    /// state — the matched activity's updated state arrives as a `data:`
    /// frame on the open stream.
    #[tokio::test]
    async fn descend_answers_with_match_report_and_stream_frame() {
        let (app, store, _registry, live) = live_app().await;
        let cookie = user_cookie(&store).await;

        // Seed one activity in the shared db (the executor, the handler, and
        // the assertions all observe it): the first CSV row matches it by
        // the local minute, the second has no activity at its time.
        {
            let mut conn = live.mem().lock().unwrap().begin();
            let act = Activity {
                id: ActivityId::new(1),
                user_id: UserId::from(1),
                what: tb_domain::ActTypeId::from(1),
                name: "Morning Ride".to_string(),
                start: time::macros::datetime!(2023-05-18 22:13:20 UTC),
                duration: 3600,
                time: Some(3500),
                distance: Some(50000),
                climb: Some(500),
                descend: None,
                energy: Some(1000),
                gear: None,
                device_name: None,
                external_id: None,
            };
            ActivityStore::activity_create(&mut conn, act)
                .await
                .unwrap();
            conn.commit().await.unwrap();
        }

        // The stream opens with a valid session (spawns the executor).
        let (status, _headers, mut body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);

        // The CSV: one matching row, one unmatched row.
        let csv = "Date,Title,Total Descent\n2023-05-18 22:13:20,Morning Ride,900\n2023-06-01 08:00:00,Phantom Ride,500\n";
        let (status, _headers, resp) =
            run_json(app, Method::POST, "/api/activ/descend", Some(&cookie), csv).await;
        assert_eq!(status, StatusCode::OK);
        let report: serde_json::Value = serde_json::from_slice(&resp).expect("report is json");
        assert_eq!(
            report["good"],
            serde_json::json!(["Morning Ride at 2023-05-18 22:13:20"]),
            "report: {report:?}"
        );
        assert_eq!(
            report["bad"],
            serde_json::json!(["Phantom Ride at 2023-06-01 08:00:00"]),
            "report: {report:?}"
        );

        // The state change arrives as a `data:` frame on the open stream
        // (the write rides the executor; the frame carries the Summary).
        let frames = read_sse_frames(&mut body, 3, Duration::from_secs(3)).await;
        assert!(
            frames
                .iter()
                .any(|f| f.contains("data:") && f.contains("Morning Ride")),
            "expected the updated activity in a data frame, got: {frames:?}"
        );
    }

    /// `GET /api/activ/rescan` (admin) stays a direct domain op (the spec
    /// §3/§6.4 carve-out), but after its commit it stops all live executors
    /// (issue #446): the open stream ends (the client's native reconnect +
    /// catch-up snapshot re-hydrates it), the answer is `204`, and the next
    /// write respawns a fresh executor.
    #[tokio::test]
    async fn rescan_answers_204_and_stops_streams() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let admin = admin_cookie(&store).await;

        // The stream opens with a valid session (spawns the user's executor).
        let (status, _headers, mut body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);

        // The rescan commits and stops all streams.
        let (status, _headers, _resp) =
            run(app.clone(), Method::GET, "/api/activ/rescan", Some(&admin)).await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        // The open stream ends (the SSE body closes).
        assert_stream_ends(&mut body).await;

        // A subsequent write respawns a fresh executor and works.
        let (status, _headers, _resp) = run_json(
            app.clone(),
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Chain"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // And a new stream opens on the respawned executor.
        let (status, _headers, _body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
    }

    /// A panicked executor task records `Dead` (spec §4.4/§4.6): the next
    /// write respawns a fresh executor, and the stream reopens on a new
    /// connection.
    #[tokio::test]
    async fn executor_panic_records_dead_and_respawns() {
        let (app, store, registry, live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let user = UserId::from(1);
        // Open a stream: the executor spawns and stays running while the
        // stream is attached (a stream-less executor reaps right after its
        // write, so the stream is what pins it to `Running` here).
        let (status, _headers, _body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        wait_for_status(&registry, user, ExecutorStatus::Running).await;
        // Force a panic on the executor's next Strava-queue read, then
        // interrupt its idle wait with a write: the write is processed, and
        // the probe after it panics the task.
        live.arm_panic();
        let (status, _headers, _body) = run_json(
            app.clone(),
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Chain"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        // The watchdog records the panic as `Dead` (not a stuck `Running`).
        wait_for_status(&registry, user, ExecutorStatus::Dead).await;
        // The next write respawns a fresh executor and succeeds.
        let (status, _headers, _body) = run_json(
            app.clone(),
            Method::POST,
            "/api/part",
            Some(&cookie),
            &part_body("Derailleur"),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        // And the stream reopens (respawning the executor for the
        // connection, which pins it to `Running` again).
        let (status, _headers, _body) = run_sse(app, "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        wait_for_status(&registry, user, ExecutorStatus::Running).await;
    }
}
