//! This module contains the implementation of the OAuth flow for
//! intervals.icu authentication.
//!
//! It mirrors the Strava driver's `oauth` module: the `/icu/login`
//! redirect (to intervals.icu's authorize URL) and the `/icu/token` exchange
//! (receive the code, exchange it, upsert `icu_users`, set the session
//! cookie). The CSRF state handling is the same HMAC scheme as the Strava
//! driver (provider-agnostic).

use anyhow::Context;
use tower_sessions::Session;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use log::{debug, warn};
use oauth2::{AuthorizationCode, Scope, TokenResponse};
use serde::{Deserialize, Serialize};

use crate::{
    appstate::AppState,
    error::AppError,
    oauth_common::{gentoken, getpath},
};
use tb_domain::Error;
use tb_provider::icu::client::{HTTP_CLIENT, ICU_CLIENT, IcuAthleteInfo, SCOPES};

#[derive(Deserialize)]
pub(crate) struct PathParam {
    inner: Option<String>,
}

pub(crate) async fn icu_auth(Query(path): Query<PathParam>) -> impl IntoResponse {
    let path = path.inner.unwrap_or("/".to_owned());
    let (auth_url, _csrf_token) = ICU_CLIENT
        .authorize_url(|| gentoken(path))
        .add_scope(Scope::new(SCOPES.to_string()))
        .url();

    // Redirect to intervals.icu's oauth service
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

    // Get an auth token (the outbound exchange — a `tb_provider::icu` op, no
    // seam)
    let token = ICU_CLIENT
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&*HTTP_CLIENT)
        .await
        .context("token exchange failed")?;

    // The service API's direct op: the one upsert transaction
    // (`IcuUser::upsert` + `is_admin`), returning the identity and the admin
    // flag. The wire-shape parsing stays in axum, beside the exchange it feeds.
    let athlete: IcuAthleteInfo = token
        .extra_fields()
        .athlete
        .clone()
        .ok_or(Error::BadRequest(
            "No Athlete Info from intervals.icu".to_string(),
        ))?;

    // intervals.icu tokens are durable: there is no refresh token. The
    // access token is stored as-is.
    let access_token = token.access_token().secret().to_string();
    let (user_id, is_admin) = state
        .icu
        .icu_login_from_token(athlete, access_token)
        .await?;

    // The tower-session write stays in axum: the token is stored
    // in `icu_users`, not the cookie.
    super::super::strava::RequestSession::create_from_token(user_id, is_admin, session).await?;

    debug!("Redirecting to {path}");
    Ok(Redirect::to(&path))
}
