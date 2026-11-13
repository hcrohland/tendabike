//! This module contains the implementation of the OAuth flow for Strava
//! authentication.
//!
//! The web shell keeps only the protocol (ADR-0014): the `/strava/login`
//! redirect (the CSRF state is generated here, beside the other protocol
//! secrets; the authorize URL is the `Provider` enum's `authorize_url`) and
//! the `/strava/token` callback (the query values are extracted and
//! forwarded opaquely to the provider service's `login` op — the scope
//! check, the token exchange, and the wire-shape parsing are the provider
//! layer's work).

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use log::{error, warn};
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

pub(crate) async fn strava_auth(Query(path): Query<PathParam>) -> impl IntoResponse {
    let path = path.inner.unwrap_or("/".to_owned());
    // The web shell generates the CSRF state (beside its other protocol
    // secrets); the enum assembles the authorize URL from the driver's
    // client config (ADR-0014).
    let url = Provider::Strava.authorize_url(gentoken(path).secret());

    // Redirect to Strava's oauth service
    Redirect::to(&url)
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
    login_and_redirect(state, session, Provider::Strava, code, scope, path).await
}

pub(crate) async fn logout(session: Session) -> impl IntoResponse {
    if session.delete().await.is_err() {
        error!("Failed to destroy session");
    }
    Redirect::to("/").into_response()
}
