//! This module contains the implementation of the OAuth flow for
//! intervals.icu authentication.
//!
//! It mirrors the Strava driver's `oauth` module (ADR-0014): the `/icu/login`
//! redirect (the CSRF state is generated in the web shell, beside its other
//! protocol secrets; the authorize URL is the `Provider` enum's
//! `authorize_url`) and the `/icu/token` callback (the query values are
//! extracted and forwarded opaquely to the provider service's `login` op —
//! the scope check, the token exchange, and the wire-shape parsing are the
//! provider layer's work). The CSRF state handling is the same HMAC scheme as
//! the Strava flow (provider-agnostic).

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
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

#[derive(Deserialize)]
pub(crate) struct PathParam {
    inner: Option<String>,
}

pub(crate) async fn icu_auth(Query(path): Query<PathParam>) -> impl IntoResponse {
    let path = path.inner.unwrap_or("/".to_owned());
    // The web shell generates the CSRF state (beside its other protocol
    // secrets); the enum assembles the authorize URL from the driver's
    // client config (ADR-0014).
    let url = Provider::Icu.authorize_url(gentoken(path).secret());

    // Redirect to intervals.icu's oauth service
    Redirect::to(&url)
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
    let (code, scope, path) = match query {
        AuthResponse::Error { error, .. } => {
            warn!("Authentication failed with error: {error}");
            return Ok(Redirect::to("/"));
        }
        // intervals.icu omits the `scope` query param on some callbacks: an
        // absent scope is the empty string (the provider layer accepts it).
        AuthResponse::Request { code, state, scope } => {
            (code, scope.unwrap_or_default(), getpath(state)?)
        }
    };
    login_and_redirect(state, session, Provider::Icu, code, scope, path).await
}
