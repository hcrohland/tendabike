mod oauth;
mod webhook;
mod redirect;
use axum::{Router, routing::{get, post}};
pub(crate) use oauth::*;

pub(crate) fn router(state: crate::AppState) -> Router{
    Router::new()
        .route("/login", get(oauth::strava_auth))
        .route("/token", get(oauth::login_authorized))
        .route("/logout", get(oauth::logout))
        .route("/hooks", get(webhook::hooks))
        .route("/callback",  post(webhook::create_event).get(webhook::validate_subscription))
        .route("/sync", get(webhook::sync_api))
        .route("/bikes/:id", get(redirect::redirect_gear))
        .route("/activities/:id", get(redirect::redirect_act))
        .route("/users/:id", get(redirect::redirect_user))
        .with_state(state)
}