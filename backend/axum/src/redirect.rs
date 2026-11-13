//! Provider-neutral redirect routes (ADR-0014): the gear and user redirect
//! direct ops, shared by the per-provider route tables. Each handler drives
//! the neutral `provider` handle with the route's provider — the per-provider
//! route tables stay per-provider (they are genuinely different), the handler
//! logic is one. The gear op's typed absence (no provider page — icu gear is
//! a private settings item, a manual part has no provider id) is a no-op
//! redirect to the current page (the empty relative reference); a failed op
//! goes home (`/`).

use axum::{
    extract::{Path, State},
    response::Redirect,
};
use tb_domain::PartId;
use tb_provider::Provider;

use crate::{RequestSession, appstate::AppState, error::AppError};

/// The gear redirect direct op (ADR-0014): the neutral `provider` handle's
/// `gear_url` op with the route's provider. A present URL permanent-redirects,
/// the typed absence (no provider page) is a no-op redirect to the current
/// page (the empty relative reference), and a failed op goes home (`/`).
async fn gear_redirect_inner(
    provider: Provider,
    user: RequestSession,
    id: i32,
    state: AppState,
) -> Result<Redirect, AppError> {
    let result = state
        .provider
        .gear_url(provider, Box::new(user), PartId::from(id))
        .await;
    match result {
        Ok(None) => Ok(Redirect::permanent("")),
        Ok(Some(uri)) => Ok(Redirect::permanent(&uri)),
        Err(_) => Ok(Redirect::permanent("/")),
    }
}

/// The user redirect direct op (ADR-0014): the neutral `provider` handle's
/// `user_url` op with the route's provider (no session: the route has no user
/// context to gate on). A present URL permanent-redirects; a failed op goes
/// home (`/`).
async fn user_redirect_inner(
    provider: Provider,
    id: i32,
    state: AppState,
) -> Result<Redirect, AppError> {
    let uri = state
        .provider
        .user_url(provider, id.into())
        .await
        .unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::permanent(&uri))
}

/// The Strava gear redirect route (`/strava/bikes/{id}`).
pub(crate) async fn strava_gear_redirect(
    user: RequestSession,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    gear_redirect_inner(Provider::Strava, user, id, state).await
}

/// The icu gear redirect route (`/icu/bikes/{id}`): icu gear has no public
/// page, so the op is the typed absence and the route is a no-op.
pub(crate) async fn icu_gear_redirect(
    user: RequestSession,
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    gear_redirect_inner(Provider::Icu, user, id, state).await
}

/// The Strava user redirect route (`/strava/users/{id}`).
pub(crate) async fn strava_user_redirect(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    user_redirect_inner(Provider::Strava, id, state).await
}

/// The icu user redirect route (`/icu/users/{id}`): the public athlete
/// profile page (a new capability — the icu arm builds it from the athlete id
/// in the icu user row).
pub(crate) async fn icu_user_redirect(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    user_redirect_inner(Provider::Icu, id, state).await
}
