//! This module contains the implementation of the intervals.icu API endpoints
//! using the Axum web framework. It mirrors the Strava driver's `strava`
//! module: the OAuth connect flow (`/icu/login`, `/icu/token`), the webhook
//! (`/icu/callback`, spec #764), and the onboarding backfill trigger
//! (`/icu/onboarding/sync`, spec #766); the redirect routes land in later
//! tickets.

use axum::{
    Router,
    routing::{get, post},
};

use crate::appstate::AppState;

mod oauth;
mod onboarding;
mod webhook;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(oauth::icu_auth))
        .route("/token", get(oauth::login_authorized))
        .route("/callback", post(webhook::create_event))
        .route("/onboarding/sync", post(onboarding::trigger_initial_sync))
}
