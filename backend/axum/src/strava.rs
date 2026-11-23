//! This module contains the implementation of the Strava API endpoints using the Axum web framework.
//! It includes endpoints for authentication and webhooks.
//! The module also exports the `oauth` module for use in other parts of the application.
//!
//! The endpoints are defined using the `Router` from Axum and are mounted on the `/login`, `/token`, and `/callback` routes.
//!
//! The `router` function takes an `AppState` as an argument and returns a `Router` with the mounted endpoints and the provided state.
//!

use axum::{
    Router,
    routing::{get, post},
};

use crate::appstate::AppState;

mod oauth;

mod webhook;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(oauth::strava_auth))
        .route("/token", get(oauth::login_authorized))
        .route(
            "/callback",
            post(webhook::create_event).get(webhook::validate_subscription),
        )
}
