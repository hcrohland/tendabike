//! This module contains the implementation of the Strava API endpoints using the Axum web framework.
//! It includes endpoints for authentication, webhooks, and redirects.
//! The module also exports the `oauth` module for use in other parts of the application.
//!
//! The endpoints are defined using the `Router` from Axum and are mounted on the `/login`, `/token`, `/logout`, `/hooks`, `/callback`, `/sync`, `/sync/:id`, `/bikes/:id`, `/activities/:id`, and `/users/:id` routes.
//!
//! The `router` function takes an `AppState` as an argument and returns a `Router` with the mounted endpoints and the provided state.
//!

mod oauth;
mod redirect;
mod requestuser;
mod webhook;
use axum::{
    routing::{get, post},
    Router,
};
pub(crate) use oauth::*;
pub(crate) use requestuser::*;

use crate::appstate::AppState;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(oauth::strava_auth))
        .route("/token", get(oauth::login_authorized))
        .route("/logout", get(oauth::logout))
        .route("/hooks", get(webhook::hooks))
        .route(
            "/callback",
            post(webhook::create_event).get(webhook::validate_subscription),
        )
        .route("/sync", get(webhook::sync_api))
        .route("/sync/:id", get(webhook::sync))
        .route("/bikes/:id", get(redirect::redirect_gear))
        .route("/activities/:id", get(redirect::redirect_act))
        .route("/users/:id", get(redirect::redirect_user))
        .route("/disable/:id", post(crate::user::revoke_user))
}
