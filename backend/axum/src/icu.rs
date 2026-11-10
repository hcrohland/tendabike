//! This module contains the implementation of the intervals.icu API endpoints
//! using the Axum web framework. It mirrors the Strava driver's `strava`
//! module (spec #763): this ticket lands the OAuth connect flow
//! (`/icu/login`, `/icu/token`); the webhook, sync, and redirect routes land
//! in later tickets.

use axum::{Router, routing::get};

use crate::appstate::AppState;

mod oauth;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(oauth::icu_auth))
        .route("/token", get(oauth::login_authorized))
}
