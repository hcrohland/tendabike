//! This module contains the implementation of the OAuth flow for Strava authentication.
//! It defines the `oauth_client` function which returns a `StravaClient` that can be used to
//! authenticate users with Strava.
//!
//! It also defines the `StravaAthleteInfo` and `StravaExtraTokenFields` structs which are used
//! to store additional information about the authenticated user.
//!
//! Finally, it defines the `COOKIE_NAME` constant which is used to store the session cookie.

use anyhow::Context;
use tower_sessions::Session;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use log::{debug, error, warn};
use oauth2::{AuthorizationCode, Scope, TokenResponse};
use serde::{Deserialize, Serialize};

use crate::{
    appstate::AppState,
    error::AppError,
    oauth_common::{gentoken, getpath},
};
use tb_domain::Error;
use tb_provider::strava::client::{HTTP_CLIENT, SCOPES, STRAVACLIENT, StravaAthleteInfo};

#[derive(Deserialize)]
pub(crate) struct PathParam {
    inner: Option<String>,
}

pub(crate) async fn strava_auth(Query(path): Query<PathParam>) -> impl IntoResponse {
    let path = path.inner.unwrap_or("/".to_owned());
    let (auth_url, _csrf_token) = STRAVACLIENT
        .authorize_url(|| gentoken(path))
        .add_scope(Scope::new(SCOPES.to_string()))
        .url();

    // Redirect to Strava's oauth service
    Redirect::to(auth_url.as_ref())
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
    let (code, path) = match query {
        AuthResponse::Error { error, .. } => {
            warn!("Authentication failed with error: {error}");
            return Ok(Redirect::to("/"));
        }
        AuthResponse::Request { code, state, scope } => {
            if scope != SCOPES {
                warn!("Insufficient authorization {scope}");
                return Ok(Redirect::to("/"));
            }
            (code, getpath(state)?)
        }
    };

    // Get an auth token (the outbound exchange — a `tb_provider::strava` op, no seam)
    let token = STRAVACLIENT
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&*HTTP_CLIENT)
        .await
        .context("token exchange failed")?;

    // The service API's direct op (spec #558 §5.2): the one upsert
    // transaction (`StravaUser::upsert` + `is_admin`), returning the
    // identity and the admin flag. The wire-shape parsing stays in axum,
    // beside the exchange it feeds.
    let mut athlete: StravaAthleteInfo = token
        .extra_fields()
        .athlete
        .clone()
        .ok_or(Error::BadRequest("No Athlete Info from Strava".to_string()))?;

    // make sure you have a non-local url
    if let Some(a) = &athlete.avatar
        && !a.starts_with("http")
    {
        athlete.avatar = None;
    }
    let refresh_token = token.refresh_token().cloned();
    let (user_id, is_admin) = state
        .provider
        .login_from_token(athlete, refresh_token)
        .await?;

    // The tower-session write stays in axum (spec #558 §5.2): the token
    // flows through the `tb_provider::strava` `TokenCache`, not the cookie.
    super::RequestSession::create_from_token(user_id, is_admin, session).await?;

    debug!("Redirecting to {path}");
    Ok(Redirect::to(&path))
}

pub(crate) async fn logout(session: Session) -> impl IntoResponse {
    if session.delete().await.is_err() {
        error!("Failed to destroy session");
    }
    Redirect::to("/").into_response()
}
