//! This module contains the implementation of the OAuth flow for Strava authentication.
//! It defines the `oauth_client` function which returns a `StravaClient` that can be used to
//! authenticate users with Strava.
//!
//! It also defines the `StravaAthleteInfo` and `StravaExtraTokenFields` structs which are used
//! to store additional information about the authenticated user.
//!
//! Finally, it defines the `COOKIE_NAME` constant which is used to store the session cookie.

use anyhow::Context;
use async_session::{
    base64, hmac,
    log::{debug, warn},
    sha2, MemoryStore, Session, SessionStore,
};
use axum::{
    extract::{Query, State},
    http::{header::SET_COOKIE, HeaderMap},
    response::{IntoResponse, Redirect},
};
use axum_extra::TypedHeader;
use http::StatusCode;
use oauth2::{
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, ExtraTokenFields,
    RedirectUrl, RevocationUrl, Scope, StandardRevocableToken, StandardTokenResponse,
    TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::env;

use crate::error::AppError;
use tb_domain::{Error, TbResult};
use tb_strava::StravaId;

pub(crate) static COOKIE_NAME: &str = "SESSION";

lazy_static::lazy_static! {
    pub(super) static ref STRAVACLIENT: StravaClient = strava_oauth_client();
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StravaAthleteInfo {
    pub id: StravaId,
    pub firstname: String,
    pub lastname: String,
    #[serde(flatten)]
    other: serde_json::Value,
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
    // profile_medium: String,
    // profile: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StravaExtraTokenFields {
    pub athlete: Option<StravaAthleteInfo>,
}

impl ExtraTokenFields for StravaExtraTokenFields {}

type StravaTokenResponse = StandardTokenResponse<StravaExtraTokenFields, BasicTokenType>;

pub(crate) type StravaClient = Client<
    BasicErrorResponse,
    StravaTokenResponse,
    BasicTokenType,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
>;

pub(crate) fn strava_oauth_client() -> StravaClient {
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

    StravaClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        .set_auth_type(oauth2::AuthType::RequestBody)
        .set_redirect_uri(redirect_url)
        .set_revocation_uri(revocation_url)
}

lazy_static::lazy_static! {
    static ref CSRF_KEY: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();
}

fn hmac_signature(key: &[u8], msg: &str) -> String {
    use hmac::{Hmac, Mac, NewMac};
    type HmacSha256 = Hmac<sha2::Sha256>;

    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(msg.as_bytes());
    let signature = mac.finalize().into_bytes();

    base64::encode(signature)
}

fn gentoken(path: String) -> CsrfToken {
    let random: Vec<u8> = (0..16).map(|_| rand::random()).collect();
    let random = base64::encode_config(random, base64::URL_SAFE_NO_PAD);
    let msg = path + "+" + &random;
    let sig = hmac_signature(&CSRF_KEY, &msg);

    CsrfToken::new(msg + ":" + &sig)
}

fn getpath(state: String) -> TbResult<String> {
    let msg = state.split(':').collect::<Vec<_>>();
    if msg.len() != 2 || hmac_signature(&CSRF_KEY, msg[0]) != msg[1] {
        return Err(Error::BadRequest(format!("CSRF token failure: {state}")));
    };
    Ok(msg[0].split('+').next().unwrap_or("").to_owned())
}

#[derive(Deserialize)]
pub struct PathParam {
    path: String,
}

pub(crate) async fn strava_auth(path: Option<Query<PathParam>>) -> impl IntoResponse {
    let path = match path {
        None => "/".to_owned(),
        Some(Query(p)) => p.path,
    };
    let (auth_url, _csrf_token) = STRAVACLIENT
        .authorize_url(|| gentoken(path))
        .add_scope(Scope::new(
            "read,activity:read_all,profile:read_all".to_string(),
        ))
        .url();

    // Redirect to Strava's oauth service
    Redirect::to(auth_url.as_ref())
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct AuthRequest {
    code: String,
    state: String,
    // scope: String,
}

pub(crate) async fn login_authorized(
    Query(query): Query<AuthRequest>,
    State(memstore): State<MemoryStore>,
    State(store): State<crate::DbPool>,
) -> Result<(HeaderMap, Redirect), AppError> {
    let AuthRequest { code, state } = query;

    let path = getpath(state)?;

    let mut store = store.get().await?;

    // Get an auth token
    let token = STRAVACLIENT
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
        .context("token exchange failed")?;

    if token.scopes().is_some() {
        warn!("Insufficient authorization {:?}", token.scopes());
        Err(Error::NotAuth(format!(
            "Insufficient authorization {:?}",
            token.scopes()
        )))?
    }

    let user = super::RequestUser::create_from_token(token, &mut store).await?;

    // Create a new session filled with user data
    let mut session = Session::new();
    session
        .insert("user", &user)
        .context("session insert failed")?;

    // Store session and get corresponding cookie
    let cookie = match memstore.store_session(session).await? {
        Some(cookie) => cookie,
        None => Err(Error::AnyFailure(anyhow::anyhow!(
            "Failed to store session".to_string(),
        )))?,
    };

    // Build the cookie
    let cookie = format!("{}={}; SameSite=Lax; Path=/", COOKIE_NAME, cookie);

    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    debug!("Redirecting to {path}");
    Ok((headers, Redirect::to(&path)))
}

// Valid user session required. If there is none, redirect to the auth page
pub(crate) async fn logout(
    State(memstore): State<MemoryStore>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> impl IntoResponse {
    if let Some(cookie) = cookies.get(COOKIE_NAME) {
        let session = match memstore.load_session(cookie.to_string()).await {
            Ok(Some(s)) => s,
            // No session active, just redirect
            _ => return Redirect::to("/").into_response(),
        };
        if memstore.destroy_session(session).await.is_err() {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to destroy session",
            )
                .into_response();
        }
    }

    Redirect::to("/").into_response()
}
