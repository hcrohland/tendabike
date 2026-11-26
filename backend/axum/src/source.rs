//! Provider-neutral redirect routes.
//!
//! `/source/activities/{id}` redirects to the activity's provider URL. The
//! id is provider-prefixed (`strava:12345` / `icu:i55751783`), and the
//! neutral `provider` handle's `activity_url` op is keyed by the provider
//! parsed from that prefix (the single home for the prefix knowledge,
//! ADR-0013).
//!
//! `/source/gear/{id}` redirects to the part's provider URL, and
//! `/source/users/{id}` to the user's provider profile URL. In both the
//! provider is the target's own, derived in the op (ADR-0014): the routes
//! stay provider-ignorant.

use axum::{
    Router,
    extract::{Path, State},
    response::Redirect,
    routing::get,
};
use tb_domain::{ActivityId, PartId, UserId};

use crate::{RequestSession, appstate::AppState, error::AppError};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/activities/{id}", get(redirect_act))
        .route("/gear/{id}", get(redirect_gear))
        .route("/users/{id}", get(redirect_user))
}

async fn redirect_act(
    user: RequestSession,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    // The redirect direct op (ADR-0014): the neutral `provider` handle's
    // `activity_url` op, which keys on the provider parsed from the id's
    // prefix (the single home for the prefix knowledge, ADR-0013) and checks
    // ownership. The handler maps `Err` (and an unparseable prefix) → a
    // permanent redirect to "/".
    let activity_id = ActivityId::from(id);
    let uri = state
        .provider
        .activity_url(Box::new(user), activity_id)
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

/// The gear redirect direct op (ADR-0014): the neutral `provider` handle's
/// `gear_url` op (the part read is the ownership check). A present URL
/// permanent-redirects, the typed absence (no provider page — icu gear, a
/// manual part) is a no-op redirect to the current page (the empty relative
/// reference), and a failed op goes home (`/`).
async fn redirect_gear(
    user: RequestSession,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let result = state
        .provider
        .gear_url(Box::new(user), PartId::from(id))
        .await;
    match result {
        Ok(None) => Ok(Redirect::permanent("")),
        Ok(Some(uri)) => Ok(Redirect::permanent(&uri)),
        Err(_) => Ok(Redirect::permanent("/")),
    }
}

/// The user redirect direct op (ADR-0014): the neutral `provider` handle's
/// `user_url` op (no session: the route has no user context to gate on). A
/// present URL permanent-redirects; a failed op goes home (`/`).
async fn redirect_user(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let uri = state
        .provider
        .user_url(UserId::from(id))
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}
