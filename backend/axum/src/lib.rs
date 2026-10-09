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

    use tb_domain::{PartStore, Summary, UserId};

    use crate::stream::Registry;
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
        // Give the executor a moment to spawn and register this stream.
        tokio::time::sleep(Duration::from_millis(100)).await;
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
    /// executor then consumes the queued sync (no refresh token in the
    /// in-memory seam: `NotAuth` disables the user and clears the queue).
    #[tokio::test]
    async fn onboarding_sync_returns_200_and_user() {
        let (app, store, _registry, live) = live_app().await;
        let cookie = user_cookie(&store).await;
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

    /// A write enqueued on the user's executor pushes its `Summary` to the
    /// user's SSE stream.
    #[tokio::test]
    async fn write_pushes_summary_frame_to_stream() {
        let (app, store, _registry, _live) = live_app().await;
        let cookie = user_cookie(&store).await;
        let (status, _headers, mut body) =
            run_sse(app.clone(), "/api/user/stream", Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK);
        // Let the executor spawn and register this stream.
        tokio::time::sleep(Duration::from_millis(100)).await;
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
}
