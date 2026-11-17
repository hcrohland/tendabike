//! The provider availability endpoint (ADR-0016):
//! `GET /api/providers` — unauthenticated, the enabled providers as an
//! ordered list of wire values. The connect screen (the anonymous landing
//! page) offers only the providers it lists, and renders the failure page
//! when the list is empty.

use axum::{Json, Router, extract::State, routing::get};

use crate::appstate::AppState;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/", get(list))
}

/// `GET /api/providers`: the enabled providers, in the
/// `Provider` enum's definition order, as wire values (`["strava"]`).
/// Unauthenticated: the response carries no user data — it is the
/// instance's provider configuration.
async fn list(State(state): State<AppState>) -> Json<Vec<String>> {
    let providers = state.provider.enabled_providers();
    Json(
        providers
            .iter()
            .map(|p| p.wire_value().to_string())
            .collect(),
    )
}
