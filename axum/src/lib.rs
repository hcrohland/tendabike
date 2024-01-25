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

use async_session::MemoryStore;
use axum::Router;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tb_diesel::DbPool;

mod domain;

mod strava;
use strava::{AxumAdmin, RequestUser};

mod appstate;
use appstate::*;

mod error;
use error::*;

pub async fn start(pool: DbPool, path: std::path::PathBuf, addr: SocketAddr) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // `MemoryStore` is just used as an example. Don't use this in production.
    let store = MemoryStore::new();

    let app_state = AppState::new(store, pool);

    let app = Router::new()
        .nest_service("/", tower_http::services::ServeDir::new(path))
        .nest("/", domain::router())
        .nest("/strava", strava::router())
        .with_state(app_state)
        .fallback(error::fallback)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    tracing::debug!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
