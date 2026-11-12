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
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{
    RequestSession,
    appstate::AppState,
    error::AppError,
    oauth_common::{gentoken, getpath},
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

    // The provider-agnostic direct op (ADR-0014): one transaction — the
    // provider layer's dispatch does the provider's login work (the scope
    // check, the token exchange, the athlete wire-shape parsing, and the
    // upsert), returning the identity and the admin flag. The web shell
    // forwards the opaque callback values — no driver wire type crosses the
    // seam.
    let (user_id, is_admin) = state
        .provider
        .login(Provider::Strava, &code, &scope)
        .await?;

    // The tower-session write stays in axum (spec #558 §5.2).
    RequestSession::create_from_token(user_id, is_admin, session).await?;

    debug!("Redirecting to {path}");
    Ok(Redirect::to(&path))
}

pub(crate) async fn logout(session: Session) -> impl IntoResponse {
    if session.delete().await.is_err() {
        error!("Failed to destroy session");
    }
    Redirect::to("/").into_response()
}
