//! This module contains the implementation of the OAuth flow for Strava
//! authentication.
//!
//! The web shell keeps only the protocol (ADR-0014): the `/strava/login`
//! redirect (the CSRF state is generated here, beside the other protocol
//! secrets; the authorize URL is the driver's `authorize_url`) and
//! the `/strava/token` callback (the query values are extracted and
//! forwarded opaquely to the provider service's `login` op — the scope
//! check, the token exchange, and the wire-shape parsing are the provider
//! layer's work).

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use log::warn;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{
    appstate::AppState,
    error::AppError,
    oauth_common::{gentoken, getpath, login_and_redirect},
};
use tb_provider::Provider;
use tb_provider::driver::Provider as _;
use tb_provider::strava::StravaDriver;

#[derive(Deserialize)]
pub(crate) struct PathParam {
    inner: Option<String>,
}

/// `GET /strava/login`: the OAuth entry point. A disabled provider (its
/// client id unset, ADR-0016) answers a clear "unavailable" — no redirect,
/// no 500: the provider is hidden from the connect screen, and a direct hit
/// degrades gracefully.
pub(crate) async fn strava_auth(
    Query(path): Query<PathParam>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    if !state
        .provider
        .enabled_providers()
        .contains(&Provider::Strava)
    {
        // The provider layer owns the disabled-provider message (ADR-0016).
        return Err(tb_provider::disabled(Provider::Strava).into());
    }
    let path = path.inner.unwrap_or("/".to_owned());
    // The web shell generates the CSRF state (beside its other protocol
    // secrets); the driver assembles the authorize URL from its client
    // config (ADR-0014). The Strava flow is a plain
    // authorization-code exchange: no PKCE challenge.
    let url = StravaDriver::authorize_url(gentoken(path).secret(), None);

    // Redirect to Strava's oauth service
    Ok(Redirect::to(&url))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum AuthResponse {
    Request {
        code: String,
        state: String,
        scope: String,
    },
    Error {
        error: String,
        state: String,
    },
}

pub(crate) async fn login_authorized(
    Query(query): Query<AuthResponse>,
    session: Session,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    let (code, scope, path) = match query {
        AuthResponse::Error { error, .. } => {
            warn!("Authentication failed with error: {error}");
            return Ok(Redirect::to("/"));
        }
        AuthResponse::Request { code, state, scope } => (code, scope, getpath(state)?),
    };
    // The Strava flow is a plain authorization-code exchange: no PKCE
    // verifier.
    login_and_redirect(state, session, Provider::Strava, code, scope, path, None).await
}
