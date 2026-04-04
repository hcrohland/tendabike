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
use oauth2::{
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, EndpointNotSet,
    EndpointSet, ExtraTokenFields, RedirectUrl, RevocationUrl, Scope, StandardRevocableToken,
    StandardTokenResponse, TokenUrl,
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    reqwest,
};
use serde::{Deserialize, Serialize};
use std::{env, sync::LazyLock};

use crate::error::AppError;
use tb_domain::{Error, Store, TbResult};
use tb_strava::StravaId;

pub(super) static STRAVACLIENT: LazyLock<StravaClient> = LazyLock::new(strava_oauth_client);
pub(super) static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(http_client);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StravaAthleteInfo {
    pub id: StravaId,
    pub firstname: String,
    pub lastname: String,
    #[serde(rename = "profile_medium")]
    pub avatar: Option<String>,
    // profile: String,
    // bio: String,
    // city: String,
    // state: String,
    // country: String,
    // sex: String,
    // premium: bool,
    // summit: bool,
    // created_at: String,
    // updated_at: String,
    // badge_type_id: i32,
    // weight: f32,
    #[serde(flatten)]
    other: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StravaExtraTokenFields {
    pub athlete: Option<StravaAthleteInfo>,
}

impl ExtraTokenFields for StravaExtraTokenFields {}

pub(crate) type StravaTokenResponse = StandardTokenResponse<StravaExtraTokenFields, BasicTokenType>;

pub(crate) type StravaClient<
    HasAuthUrl = EndpointSet,
    HasDeviceAuthUrl = EndpointNotSet,
    HasIntrospectionUrl = EndpointNotSet,
    HasRevocationUrl = EndpointSet,
    HasTokenUrl = EndpointSet,
> = Client<
    BasicErrorResponse,
    StravaTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    HasAuthUrl,
    HasDeviceAuthUrl,
    HasIntrospectionUrl,
    HasRevocationUrl,
    HasTokenUrl,
>;

pub fn strava_oauth_client() -> StravaClient {
    // Environment variables (* = required):
    // *"CLIENT_ID"     "REPLACE_ME";
    // *"CLIENT_SECRET" "REPLACE_ME";
    //  "REDIRECT_URL"  "http://127.0.0.1:3000/auth/authorized";
    //  "AUTH_URL"      "https://www.strava.com/oauth/authorize";
    //  "TOKEN_URL"     "https://www.strava.com/oauth/token";

    let client_id = ClientId::new(env::var("CLIENT_ID").expect("Missing CLIENT_ID!"));
    let client_secret =
        ClientSecret::new(env::var("CLIENT_SECRET").expect("Missing CLIENT_SECRET!"));

    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:8000/strava/token".to_string());
    let redirect_url = RedirectUrl::new(redirect_url).unwrap();

    let auth_url = AuthUrl::new("https://www.strava.com/oauth/authorize".to_string()).unwrap();
    let token_url = TokenUrl::new("https://www.strava.com/oauth/token".to_string()).unwrap();
    let revocation_url =
        RevocationUrl::new("https://www.strava.com/oauth/deauthorize".to_string()).unwrap();

    StravaClient::new(client_id)
        .set_client_secret(client_secret)
        .set_auth_uri(auth_url)
        .set_token_uri(token_url)
        .set_auth_type(oauth2::AuthType::RequestBody)
        .set_redirect_uri(redirect_url)
        .set_revocation_url(revocation_url)
}

fn http_client() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        // Following redirects opens the client up to SSRF vulnerabilities.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("Client should build")
}

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

const SCOPES: &str = "read,activity:read_all,profile:read_all";

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
    State(store): State<crate::DbPool>,
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

    // Get an auth token
    let token = STRAVACLIENT
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&*HTTP_CLIENT)
        .await
        .context("token exchange failed")?;

    let mut conn = store.begin().await?;
    super::RequestSession::create_from_token(token, session, &mut conn).await?;
    conn.commit().await?;

    debug!("Redirecting to {path}");
    Ok(Redirect::to(&path))
}

pub(crate) async fn logout(session: Session) -> impl IntoResponse {
    if session.delete().await.is_err() {
        error!("Failed to destroy session");
    }
    Redirect::to("/").into_response()
}
