mod oauth;
mod webhook;
use axum::{Router, routing::{get, post}};
pub(crate) use oauth::*;

pub(crate) fn router(state: crate::AppState) -> Router{
    Router::new()
        .route("/login", get(oauth::strava_auth))
        .route("/token", get(oauth::login_authorized))
        .route("/auth/check", get(crate::protected))
        .route("/auth/admin", get(crate::admin_check))
        .route("/logout", get(oauth::logout))
        .route("/hooks", get(webhook::hooks))
        .route("/callback",  post(webhook::create_event).get(webhook::validate_subscription))
        .route("/sync", get(webhook::sync_api))
        .with_state(state)
}