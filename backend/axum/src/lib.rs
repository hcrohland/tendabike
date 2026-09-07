//! This file contains the implementation of the presentation layer of the Tendabike server using the Axum framework.
//!
//! The presentation layer is responsible for handling HTTP requests and responses, and for translating them into
//! actions that the application layer can understand. The Axum framework is used to implement the presentation layer.
//!
//! This file defines the `start` function, which is the entry point for the presentation layer. It takes a database
//! connection pool, a path to the directory containing static files, and a socket address to bind to. It sets up the
//! necessary components for the presentation layer, such as the router and the middleware, and starts the server.
//!
//! This file also contains the definitions of various modules that implement the endpoints for the different resources
//! of the Tendabike server, such as users, parts, attachments, activities, and Strava integration.
//!

use anyhow::Context;
use axum::Router;
use std::net::SocketAddr;
use tb_domain::TbResult;
use tower_sessions::{ExpiredDeletion, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tb_sqlx::DbPool;

mod domain;

mod strava;
use strava::{AxumAdmin, RequestSession};

mod appstate;
use appstate::*;

mod error;
use error::*;

#[cfg(test)]
mod test_support;

fn routes(app_state: AppState) -> Router {
    Router::new()
        .nest("/api", domain::router())
        .nest("/strava", strava::router())
        .with_state(app_state)
}

pub async fn start(database_url: &str, path: std::path::PathBuf, addr: SocketAddr) -> TbResult<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = tb_sqlx::DbPool::new(database_url).await?;

    let session_store = PostgresStore::new(pool.raw());
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

    let app_state = AppState::new(pool);

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
    use axum::Router;
    use http::{Method, StatusCode};
    use tower_sessions::MemoryStore;

    use crate::test_support::{admin_cookie, run, test_app, user_cookie};

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
}
