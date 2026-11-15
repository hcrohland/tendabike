//! This file contains the implementation of the presentation layer of the Tendabike server using the Axum framework.
//!
//! The presentation layer is responsible for handling HTTP requests and responses, and for translating them into
//! actions that the application layer can understand. The Axum framework is used to implement the presentation layer.
//!
//! The web layer is a dumb presentation layer (ADR-0007, executable spec
//! #558 §7): the state and every handler carry the app's service API as
//! `Arc<dyn ServiceApi>` (the exec-owned surface), and the concrete adapter
//! (the `tb_exec::Registry` over the database pool) is injected at the
//! composition root. It names the trait, never a seam type — but it did
//! choose a `PostgresStore` for its sessions, so [`start`] also takes the
//! postgres connection (the `PgPool` the caller built its store from), and it
//! owns its own logging subscriber: the composition root wires the concrete
//! crates, not the crate-internal details.
//!
//! This file defines the `start` function, which is the entry point for the presentation layer. It takes the
//! service API handle, the provider service, an [`AxumConfig`] (the CSRF secret,
//! the static files path, and the bind address as raw strings), and the
//! `PgPool` for the session store. It sets up the necessary components for the
//! presentation layer, such as the router and the middleware, and starts the
//! server.
//!
//! This file also contains the definitions of various modules that implement the endpoints for the different resources
//! of the Tendabike server, such as users, parts, attachments, activities, and Strava integration.
//!

#![forbid(unsafe_code)]

use std::sync::Arc;

use anyhow::Context;
use axum::Router;
use sqlx::PgPool;
use tb_domain::TbResult;
use tb_exec::{ServiceApi, providerservice::ProviderService};
use tb_provider::Provider;
use tower_sessions::{ExpiredDeletion, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// The web layer's configuration: the CSRF secret (hex-encoded; `None`
/// generates a random key and warns), the static files directory, and the
/// bind address. All as raw strings — `start` does the parsing
/// (`Path::canonicalize`, `SocketAddr::parse`) internally.
#[derive(Debug, Clone)]
pub struct AxumConfig {
    pub csrf_secret: Option<String>,
    pub static_www: String,
    pub bind_addr: String,
}

mod domain;

mod icu;

mod redirect;

mod source;

mod session;
use session::{AxumAdmin, RequestSession};

mod strava;

mod appstate;
use appstate::*;

mod error;
use error::*;

mod oauth_common;

mod stream;

fn routes(app_state: AppState, static_path: Option<std::path::PathBuf>) -> Router {
    let mut router = Router::new()
        .nest("/api", domain::router())
        .nest("/source", source::router())
        .nest(Provider::Strava.route_prefix(), strava::router())
        .nest(Provider::Icu.route_prefix(), icu::router())
        .with_state(app_state);

    if let Some(path) = static_path {
        let fallback_file = path.join("200.html");
        let static_service = tower_http::services::ServeDir::new(path)
            .fallback(tower_http::services::ServeFile::new(fallback_file));
        router = router.fallback_service(static_service);
    }

    router
}

pub async fn start(
    api: Arc<dyn ServiceApi>,
    provider: Arc<dyn ProviderService>,
    config: AxumConfig,
    pool: PgPool,
) -> TbResult<()> {
    // Initialize the CSRF key from the config (before any request handler
    // can use it).
    crate::oauth_common::init_csrf_key(&config.csrf_secret);

    // The logging subscriber lives with the web layer it logs (the
    // composition root owns process setup, not crate-internal wiring).
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = AppState::new(api, provider);

    // Parse the config's raw strings into their typed forms.
    let path = std::path::Path::new(&config.static_www)
        .canonicalize()
        .unwrap_or_else(|_| panic!("STATIC_WWW path {} does not exist", config.static_www));
    let addr: std::net::SocketAddr = config
        .bind_addr
        .parse()
        .unwrap_or_else(|_| panic!("BIND_ADDR '{}' could not be parsed", config.bind_addr));

    // The web layer chose a `PostgresStore` for its sessions, so the
    // caller hands over the postgres connection — the same pool the
    // service API's registry was built from.
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

    let app = routes(app_state, Some(path))
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
mod lib_tests;
