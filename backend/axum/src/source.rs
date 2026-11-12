//! Provider-neutral redirect routes.
//!
//! `/source/activities/{id}` redirects to the activity's provider URL. The
//! id is provider-prefixed (`strava:12345` / `icu:i55751783`), and the
//! neutral `provider` handle's `activity_url` op is keyed by the provider
//! parsed from that prefix (the single home for the prefix knowledge,
//! ADR-0013).

use axum::{
    Router,
    extract::{Path, State},
    response::Redirect,
    routing::get,
};
use tb_domain::ActivityId;
use tb_provider::{Provider, ProviderActivityId};

use crate::{RequestSession, appstate::AppState, error::AppError};

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/activities/{id}", get(redirect_act))
}

async fn redirect_act(
    user: RequestSession,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    // The redirect direct op (ADR-0014): the neutral `provider` handle's
    // `activity_url` op, keyed by the provider parsed from the id's prefix
    // (the single home for the prefix knowledge, ADR-0013), which checks
    // ownership. The handler maps `Err` (and an unparseable prefix) → a
    // permanent redirect to "/".
    let activity_id = ActivityId::from(id);
    let provider = match ProviderActivityId::try_from(activity_id.clone()) {
        Ok(ProviderActivityId::Strava(_)) => Provider::Strava,
        Ok(ProviderActivityId::Icu(_)) => Provider::Icu,
        Err(_) => return Ok(Redirect::permanent("/")),
    };
    let uri = state
        .provider
        .activity_url(provider, Box::new(user), activity_id)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}
