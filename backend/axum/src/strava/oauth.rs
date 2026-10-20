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
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::{appstate::AppState, error::AppError};
use tb_domain::{Error, TbResult};
use tb_strava::client::{HTTP_CLIENT, SCOPES, STRAVACLIENT, StravaAthleteInfo};

static CSRF_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| {
    if let Ok(secret) = std::env::var("CSRF_SECRET") {
        hex::decode(secret).expect("CSRF_SECRET must be valid hex")
    } else {
        warn!("generating random CSRF secret. This dose not work for distributed systems");
        (0..16).map(|_| rand::random::<u8>()).collect()
    }
});

fn hmac_signature(key: &[u8], msg: &str) -> String {
    use base64::prelude::*;
    use hmac::{Hmac, KeyInit, Mac};

    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(msg.as_bytes());
    let signature = mac.finalize().into_bytes();

    BASE64_STANDARD.encode(signature)
}

fn gentoken(path: String) -> CsrfToken {
    use base64::prelude::*;

    let random: Vec<u8> = (0..16).map(|_| rand::random()).collect();
    let random = BASE64_URL_SAFE_NO_PAD.encode(random);
    let msg = path + "+" + &random;
    let sig = hmac_signature(&CSRF_KEY, &msg);

    CsrfToken::new(msg + ":" + &sig)
}

fn getpath(state: String) -> TbResult<String> {
    let msg = state.split(':').collect::<Vec<_>>();
    if msg.len() != 2 || hmac_signature(&CSRF_KEY, msg[0]) != msg[1] {
        return Err(Error::BadRequest(format!(
            "Bad signature for exchange request: {state}"
        )));
    };
    Ok(msg[0].split('+').next().unwrap_or("").to_owned())
}

#[derive(Deserialize)]
pub struct PathParam {
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

    // Get an auth token (the outbound exchange — a `tb_strava` op, no seam)
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
    let (user, is_admin) = state.api.login_from_token(athlete, refresh_token).await?;

    // The tower-session write stays in axum (spec #558 §5.2): the access
    // token and expiry ride the session, not the DB.
    super::RequestSession::create_from_token(token, session, user, is_admin).await?;

    debug!("Redirecting to {path}");
    Ok(Redirect::to(&path))
}

pub(crate) async fn logout(session: Session) -> impl IntoResponse {
    if session.delete().await.is_err() {
        error!("Failed to destroy session");
    }
    Redirect::to("/").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_signature_matches_known_vector() {
        assert_eq!(
            hmac_signature(b"tendabike", "/+abc123"),
            "jNqHCYY2M852nJCKpclLqF36vPQPrFtbECZMBvgKKlI="
        );
    }

    #[test]
    fn getpath_recovers_path_from_gentoken() {
        let token = gentoken("/foo/bar".to_string());
        assert_eq!(
            getpath(token.secret().clone()).expect("valid state"),
            "/foo/bar"
        );
    }

    #[test]
    fn getpath_rejects_corrupted_signature() {
        let token = gentoken("/foo".to_string());
        let mut state = token.secret().clone();
        state.push('x');
        assert!(matches!(getpath(state), Err(Error::BadRequest(_))));
    }

    #[test]
    fn getpath_rejects_malformed_state() {
        assert!(matches!(
            getpath("no-colon-separator".to_string()),
            Err(Error::BadRequest(_))
        ));
    }
}
