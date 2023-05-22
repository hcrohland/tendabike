//! Example OAuth (Strava) implementation.
//!
//! 1) Create client_id and client_secret at <https://www.strava.com/settings/api>
//! 2) Run with the following (replacing values appropriately):
//! ```not_rust
//! CLIENT_ID=REPLACE_ME CLIENT_SECRET=REPLACE_ME cargo run -p example-oauth
//! ```

pub(crate) mod oauth;
pub(crate) mod user;

use std::net::SocketAddr;

use diesel::{r2d2::ConnectionManager, PgConnection};
use http::StatusCode;
use oauth::StravaClient;

use async_session::MemoryStore;
use axum::{
    extract::FromRef,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[tokio::main]
pub async fn start(pool: DbPool) {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oauth2,reqwest,tb_axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // `MemoryStore` is just used as an example. Don't use this in production.
    let store = MemoryStore::new();
    let oauth_client = oauth::oauth_client();

    let app_state = AppState {
        store,
        oauth_client,
        pool,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/auth/strava", get(oauth::strava_auth))
        .route("/auth/authorized", get(oauth::login_authorized))
        .route("/auth/check", get(protected))
        .route("/logout", get(oauth::logout))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
struct AppState {
    store: MemoryStore,
    oauth_client: StravaClient,
    pool: DbPool,
}

impl FromRef<AppState> for MemoryStore {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

impl FromRef<AppState> for StravaClient {
    fn from_ref(state: &AppState) -> Self {
        state.oauth_client.clone()
    }
}

impl FromRef<AppState> for DbPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

// impl FromRef<AppState> for DbPool {
//     fn from_ref(state: &AppState) -> Self {
//         state.db.clone()
//     }
// }

use user::RUser;
async fn protected(user: RUser) -> impl IntoResponse {
    format!(
        "Welcome to the protected area :)\nHere's your info:\n{:?}",
        user.0
    )
}

// Session is optional
async fn index(user: Option<RUser>) -> impl IntoResponse {
    match user {
        Some(_u) => {
            format!(
            "Hey {}! You're logged in!\nYou may now access `/protected`.\nLog out with `/logout`.",
            "you"
        )
        }
        None => "You're not logged in.\nVisit `/auth/strava` to do so.".to_string(),
    }
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn internal_any(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
