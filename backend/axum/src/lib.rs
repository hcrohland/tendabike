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
use tower_sessions::{SessionManagerLayer, session_store::ExpiredDeletion};
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

    let app = Router::new()
        .nest("/api", domain::router())
        .nest("/strava", strava::router())
        .with_state(app_state)
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
