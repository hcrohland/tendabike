//! This module contains the implementation of the intervals.icu API endpoints
//! using the Axum web framework. It mirrors the Strava driver's `strava`
//! module: the OAuth connect flow (`/icu/login`, `/icu/token`) and the
//! webhook (`/icu/callback`, spec #764); the sync and redirect routes land in
//! later tickets.

use axum::{
    Router,
    routing::{get, post},
};

use crate::appstate::AppState;

mod oauth;
mod webhook;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(oauth::icu_auth))
        .route("/token", get(oauth::login_authorized))
        .route("/callback", post(webhook::create_event))
}
