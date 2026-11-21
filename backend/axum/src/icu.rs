//! This module contains the implementation of the intervals.icu API endpoints
//! using the Axum web framework. It mirrors the Strava driver's `strava`
//! module: the OAuth connect flow (`/icu/login`, `/icu/token`), the webhook
//! (`/icu/callback`, spec #764), and the redirect routes (ADR-0014) — the
//! user profile page (a new capability) and the gear no-op (icu gear has no
//! public page). The onboarding routes are provider-neutral
//! (`/api/onboarding/…`, spec #977) and live in the domain route table.

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
        .route("/bikes/{id}", get(crate::redirect::icu_gear_redirect))
        .route("/users/{id}", get(crate::redirect::icu_user_redirect))
}
