//! Example OAuth (Strava) implementation.
//!
//! 1) Create client_id and client_secret at <https://www.strava.com/settings/api>
//! 2) Run with the following (replacing values appropriately):
//! ```not_rust
//! CLIENT_ID=REPLACE_ME CLIENT_SECRET=REPLACE_ME cargo run -p example-oauth
//! ```

pub(crate) mod strava;
pub(crate) mod user;
mod types;
use appstate::*;
mod appstate;
mod error;
use error::*;

use std::net::SocketAddr;

use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;

use async_session::MemoryStore;
use axum::{
    extract::{State},
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
type AppDbConn = State<PooledConnection<ConnectionManager<PgConnection>>>;

#[tokio::main]
pub async fn start(pool: DbPool, path: std::path::PathBuf) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oauth2,reqwest,tb_axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // `MemoryStore` is just used as an example. Don't use this in production.
    let store = MemoryStore::new();
    let oauth_client = strava::oauth_client();

    let app_state = AppState::new(store, oauth_client, pool);

    let app = Router::new()
        .nest_service("/", tower_http::services::ServeDir::new(path))
        .nest("/user", user::router(app_state.clone()))
        .nest("/types", types::router(app_state.clone()))
        // .nest("/part", part::router())
        // .nest("/part", attachment::router())
        // .nest("/activ", activity::router())
        .nest("/strava", strava::router(app_state))
        // .with_state(app_state)
        .fallback(error::fallback);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
