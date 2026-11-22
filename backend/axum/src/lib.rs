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
//! service API handle, the provider service, an [`AxumConfig`] (the CSRF
//! secret, the static files path, and the bind address as raw strings, plus
//! the public domain and the provider credentials), and the `PgPool` for the
//! session store. It composes the provider's redirect and webhook URLs from
//! the domain, initializes the provider crate's client statics (logging the
//! webhook URL per enabled provider), sets up the necessary components for
//! the presentation layer, such as the router and the middleware, and starts
//! the server.
//!
//! This file also contains the definitions of various modules that implement the endpoints for the different resources
//! of the Tendabike server, such as users, parts, attachments, activities, and Strava integration.

#![warn(unreachable_pub)]
//!

#![forbid(unsafe_code)]

use std::sync::Arc;

use anyhow::Context;
use axum::{Router, routing::get};
use sqlx::PgPool;
use tb_domain::TbResult;
use tb_exec::{HEARTBEAT_INTERVAL, ServiceApi, providerservice::ProviderService};
use tb_provider::Provider;
use tower_sessions::{ExpiredDeletion, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// The web layer's configuration: the CSRF secret (hex-encoded; `None`
/// generates a random key and warns), the static files directory, and the
/// bind address, all as raw strings — `start` does the parsing
/// (`Path::canonicalize`, `SocketAddr::parse`) internally. It also carries the
/// public domain (defaulted to the local-dev host by the composition root) and
/// the provider credentials, from which `start` assembles the
/// `tb_provider::ProviderConfig` and calls `tb_provider::init`.
#[derive(Debug, Clone)]
pub struct AxumConfig {
    pub csrf_secret: Option<String>,
    pub static_www: String,
    pub bind_addr: String,
    pub domainname: String,
    pub strava_client_id: Option<String>,
    pub strava_client_secret: Option<String>,
    pub icu_client_id: Option<String>,
    pub icu_client_secret: Option<String>,
    pub icu_webhook_secret: String,
}

mod domain;

mod icu;

mod redirect;

mod source;

mod session;
use session::{AxumAdmin, RequestSession};

mod strava;

mod appstate;
use appstate::AppState;

mod error;
use error::ApiResult;

mod oauth_common;

mod provider_urls;

mod stream;

/// The offline gate: the instance runs offline iff the
/// enabled provider set is empty (ADR-0016). A proper subset is a normal
/// mode and never trips the gate.
fn is_offline() -> bool {
    tb_provider::enabled_providers().is_empty()
}

/// The startup offline warning: once at startup, when the
/// instance runs offline (no enabled providers), log an error naming the
/// fix. A configured subset is a normal mode and logs nothing.
fn warn_if_offline() {
    if is_offline() {
        tracing::error!(
            "No activity providers are configured; the instance is running offline. Set STRAVA_CLIENT_ID or ICU_CLIENT_ID to enable activity sync."
        );
    }
}

fn routes(app_state: AppState, static_path: Option<std::path::PathBuf>) -> Router {
    let mut router = Router::new()
        .nest("/api", domain::router())
        .nest("/source", source::router())
        .route("/logout", get(oauth_common::logout))
        .nest(Provider::Strava.route_prefix(), strava::router())
        .nest(Provider::Icu.route_prefix(), icu::router())
        .with_state(app_state);

    if let Some(path) = static_path {
        let fallback_file = path.join("200.html");
        let static_service = tower_http::services::ServeDir::new(&path)
            .fallback(tower_http::services::ServeFile::new(fallback_file));
        let content_service = tower_http::services::ServeDir::new(path.join("content"));
        router = router
            .nest_service("/content", content_service)
            .fallback_service(static_service);
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

    // Compose the provider's redirect and webhook URLs from the public domain,
    // assemble the provider config, and initialize the provider crate's client
    // statics — before `warn_if_offline` (below), so the OAuth `redirect_uri`
    // and the enabled-set derivation are in place before any request.
    let redirects = provider_urls::redirect_urls(&config.domainname);
    let webhooks = provider_urls::webhook_urls(&config.domainname);
    tb_provider::init(tb_provider::ProviderConfig {
        strava_client_id: config.strava_client_id,
        strava_client_secret: config.strava_client_secret,
        strava_redirect_url: redirects.strava,
        icu_client_id: config.icu_client_id,
        icu_client_secret: config.icu_client_secret,
        icu_redirect_url: redirects.icu,
        icu_webhook_secret: config.icu_webhook_secret,
    });

    // Log the webhook URL at INFO, one line per enabled provider (a disabled
    // provider — its client id unset — yields no line).
    for provider in tb_provider::enabled_providers() {
        let url = match provider {
            Provider::Strava => &webhooks.strava,
            Provider::Icu => &webhooks.icu,
        };
        tracing::info!("{} webhook URL: {url}", provider.wire_value());
    }

    let app_state = AppState::new(api, provider, HEARTBEAT_INTERVAL);

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

    // Once at startup: warn when the instance runs offline (no enabled
    // providers) — a configured subset is a normal mode.
    warn_if_offline();

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
