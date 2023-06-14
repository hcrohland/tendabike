//! This module contains the implementation of the OAuth flow for Strava authentication.
//! It defines the `oauth_client` function which returns a `StravaClient` that can be used to
//! authenticate users with Strava.
//!
//! It also defines the `StravaAthleteInfo` and `StravaExtraTokenFields` structs which are used
//! to store additional information about the authenticated user.
//!
//! Finally, it defines the `COOKIE_NAME` constant which is used to store the session cookie.

use crate::{internal_any, internal_error};
use async_session::{MemoryStore, Session, SessionStore};
use axum::{
    extract::{Query, State, TypedHeader},
    http::{header::SET_COOKIE, HeaderMap},
    response::{IntoResponse, Redirect},
};
use http::StatusCode;
use oauth2::{
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, ExtraTokenFields,
    RedirectUrl, Scope, StandardRevocableToken, StandardTokenResponse, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::env;
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

    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID!");
    let client_secret = env::var("CLIENT_SECRET").expect("Missing CLIENT_SECRET!");
    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:8000/strava/token".to_string());

    let auth_url = env::var("AUTH_URL")
        .unwrap_or_else(|_| "https://www.strava.com/oauth/authorize".to_string());

    let token_url =
        env::var("TOKEN_URL").unwrap_or_else(|_| "https://www.strava.com/oauth/token".to_string());

    StravaClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url).unwrap(),
        Some(TokenUrl::new(token_url).unwrap()),
    )
    .set_auth_type(oauth2::AuthType::RequestBody)
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}

pub(crate) async fn strava_auth() -> impl IntoResponse {
    let (auth_url, _csrf_token) = STRAVACLIENT
        .authorize_url(CsrfToken::new_random)
        // .add_scope(Scope::new("activity:read_all,profile:read_all".to_string()))
        .add_scope(Scope::new(
            "read,activity:read_all,profile:read_all".to_string(),
        ))
        .url();

    // Redirect to Strava's oauth service
    Redirect::to(auth_url.as_ref())
}

// Valid user session required. If there is none, redirect to the auth page
pub(crate) async fn logout(
    State(store): State<MemoryStore>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> impl IntoResponse {
    if let Some(cookie) = cookies.get(COOKIE_NAME) {
        let session = match store.load_session(cookie.to_string()).await {
            Ok(Some(s)) => s,
            // No session active, just redirect
            _ => return Redirect::to("/").into_response(),
        };
        if store.destroy_session(session).await.is_err() {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to destroy session",
            )
                .into_response();
        }
    }

    Redirect::to("/").into_response()
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub(crate) struct AuthRequest {
    code: String,
    state: String,
    scope: String,
}

pub(crate) async fn login_authorized(
    Query(query): Query<AuthRequest>,
    State(store): State<MemoryStore>,
    State(conn): State<crate::DbPool>,
) -> Result<(HeaderMap, Redirect), (StatusCode, String)> {
    let mut conn = conn.get().await.map_err(internal_any)?;

    // Get an auth token
    let token = STRAVACLIENT
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(async_http_client)
        .await
        .map_err(internal_error)?;

    if token.scopes().is_some() {
        return Err((
            StatusCode::UNAUTHORIZED,
            format!("Insufficient authorization {:?}", token.scopes()),
        ));
    }

    let user = super::RequestUser::create_from_token(token, &mut conn)
        .await
        .map_err(internal_any)?;

    // Create a new session filled with user data
    let mut session = Session::new();
    session.insert("user", &user).map_err(internal_error)?;

    // Store session and get corresponding cookie
    let cookie = match store.store_session(session).await.map_err(internal_any)? {
        Some(cookie) => cookie,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to store session".to_string(),
            ))
        }
    };

    // Build the cookie
    let cookie = format!("{}={}; SameSite=Lax; Path=/", COOKIE_NAME, cookie);

    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    Ok((headers, Redirect::to("/")))
}
