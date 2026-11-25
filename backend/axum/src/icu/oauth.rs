//! This module contains the implementation of the OAuth flow for
//! intervals.icu authentication.
//!
//! It mirrors the Strava driver's `oauth` module (ADR-0014): the `/icu/login`
//! redirect (the CSRF state is generated in the web shell, beside its other
//! protocol secrets; the authorize URL is the driver's `authorize_url`) and
//! the `/icu/token` callback (the query values are
//! extracted and forwarded opaquely to the provider service's `login` op —
//! the scope check, the token exchange, and the wire-shape parsing are the
//! provider layer's work). The CSRF state handling is the same HMAC scheme as
//! the Strava flow (provider-agnostic).
//!
//! Unlike the Strava flow, the login uses PKCE (RFC 7636, S256,
//! unconditional): the web shell generates a random code verifier per login,
//! its S256 challenge goes in the authorize URL, and the verifier
//! round-trips in the signed state so the callback can submit it in the
//! token exchange.

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use log::warn;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use oauth2::PkceCodeChallenge;

use crate::{
    appstate::AppState,
    error::AppError,
    oauth_common::{gentoken_pkce, getpath_and_verifier, login_and_redirect},
};
use tb_provider::Provider;
use tb_provider::driver::Provider as _;
use tb_provider::icu::IcuDriver;

#[derive(Deserialize)]
pub(crate) struct PathParam {
    inner: Option<String>,
}

/// `GET /icu/login`: the OAuth entry point. A disabled provider (its client
/// id unset, ADR-0016) answers a clear "unavailable" — no redirect, no 500:
/// the provider is hidden from the connect screen, and a direct hit
/// degrades gracefully.
pub(crate) async fn icu_auth(
    Query(path): Query<PathParam>,
    State(state): State<AppState>,
) -> Result<Redirect, AppError> {
    if !state.provider.enabled_providers().contains(&Provider::Icu) {
        // The provider layer owns the disabled-provider message (ADR-0016).
        return Err(tb_provider::disabled(Provider::Icu).into());
    }
    let path = path.inner.unwrap_or("/".to_owned());
    // A fresh random verifier per login: it round-trips in the signed state
    // (the callback recovers it for the token exchange), and the driver
    // derives the authorize URL's S256 challenge from it.
    let (_, verifier) = PkceCodeChallenge::new_random_sha256();
    let url = IcuDriver::authorize_url(
        gentoken_pkce(path, verifier.secret()).secret(),
        Some(verifier.secret()),
    );

    // Redirect to intervals.icu's oauth service
    Ok(Redirect::to(&url))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum AuthResponse {
    Request {
        code: String,
        state: String,
        #[serde(default)]
        scope: Option<String>,
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
    let (code, scope, path, verifier) = match query {
        AuthResponse::Error { error, .. } => {
            warn!("Authentication failed with error: {error}");
            return Ok(Redirect::to("/"));
        }
        // intervals.icu omits the `scope` query param on some callbacks: an
        // absent scope is the empty string (the provider layer accepts it).
        AuthResponse::Request { code, state, scope } => {
            let (path, verifier) = getpath_and_verifier(state)?;
            (code, scope.unwrap_or_default(), path, verifier)
        }
    };
    login_and_redirect(
        state,
        session,
        Provider::Icu,
        code,
        scope,
        path,
        Some(&verifier),
    )
    .await
}
