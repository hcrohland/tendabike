mod oauth;
use axum::{Router, routing::get};
pub(crate) use oauth::*;

pub(crate) fn router(state: crate::AppState) -> Router{
    Router::new()
        .route("/login", get(oauth::strava_auth))
        .route("/token", get(oauth::login_authorized))
        .route("/auth/check", get(crate::protected))
        .route("/auth/admin", get(crate::admin_check))
        .route("/logout", get(oauth::logout))
        .with_state(state)
}