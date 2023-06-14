//! This module contains the implementation of the OAuth flow for Strava authentication.
//! It defines the `oauth_client` function which returns a `StravaClient` that can be used to
//! authenticate users with Strava.
//!
//! It also defines the `StravaAthleteInfo` and `StravaExtraTokenFields` structs which are used
//! to store additional information about the authenticated user.
//!
//! Finally, it defines the `COOKIE_NAME` constant which is used to store the session cookie.

use crate::{error::AuthRedirect, internal_any, internal_error};
use anyhow::{bail, Context};
use async_session::{
    async_trait,
    MemoryStore, Session, SessionStore,
};
use axum::{
    extract::{
        rejection::TypedHeaderRejectionReason, FromRef, FromRequestParts, Query, State, TypedHeader,
    },
    http::{header::SET_COOKIE, HeaderMap},
    response::{IntoResponse, Redirect, Response},
    RequestPartsExt,
};
use http::{header, request::Parts, StatusCode};
use oauth2::{
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenType,
    },
    reqwest::async_http_client,
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken,
    ExtraTokenFields, RedirectUrl, Scope, StandardRevocableToken,
    StandardTokenResponse, TokenResponse, TokenUrl,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{env};
use tb_domain::{AnyResult, Error, Person, UserId};
use tb_strava::{StravaId, StravaPerson, StravaStore, StravaUser};

pub(crate) static COOKIE_NAME: &str = "SESSION";

lazy_static::lazy_static! {
    static ref STRAVACLIENT: StravaClient = oauth_client();
}

const API: &str = "https://www.strava.com/api/v3";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StravaAthleteInfo {
    id: StravaId,
    pub firstname: String,
    lastname: String,
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
    athlete: Option<StravaAthleteInfo>,
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

pub(crate) fn oauth_client() -> StravaClient {
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

    let StravaAthleteInfo {
        id,
        firstname,
        lastname,
        ..
    } = token.extra_fields().athlete.as_ref().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No athlete info".to_string(),
    ))?;

    let access_token = token.access_token().secret().clone();
    let expires = token.expires_in().map(|t| t.as_secs() as i64);
    let refresh = token.refresh_token().map(|r| r.secret().clone());
    let user = StravaUser::upsert(*id, firstname, lastname, &mut conn)
        .await
        .map_err(internal_any)?;

    // get (and eventually create) StravaUser
    let user = update_user(&token, user, &mut conn).await.map_err(internal_any)?;

    let is_admin = user.tendabike_id.is_admin(&mut conn).await.map_err(internal_any)?;
    let user = RequestUser::new(
        user.tendabike_id,
        user.id,
        firstname.clone(),
        lastname.clone(),
        is_admin,
        access_token,
        expires,
        refresh,
    );
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

async fn update_user(
    token: &StandardTokenResponse<StravaExtraTokenFields, BasicTokenType>,
    user: StravaUser,
    conn: &mut impl StravaStore,
) -> AnyResult<StravaUser> {
    let refresh = token.refresh_token().map(|r| r.secret());
    conn.stravaid_update_token(user.id, refresh).await
}

/* pub(crate) async fn refresh_token(
    user: StravaUser,
    conn: &mut impl StravaStore,
) -> AnyResult<StravaUser> {
    if user.token_is_valid() {
        return Ok(user);
    }

    info!("refreshing access token for strava id {}", user.strava_id());

    ensure!(
        user.expires_at != 0,
        Error::NotAuth("User needs to authenticate".to_string())
    );
    let refresh_token = RefreshToken::new(user.refresh_token.clone());

    let tokenset = STRAVACLIENT
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await?;
    update_user(&tokenset, user, conn).await
}
 */
// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RequestUser {
    id: UserId,
    strava_id: StravaId,
    firstname: String,
    name: String,
    is_admin: bool,
    access_token: String,
    expires_at: Option<i64>,
    refresh_token: Option<String>,
}

impl RequestUser {
    pub(crate) fn new(
        id: UserId,
        strava_id: StravaId,
        firstname: String,
        name: String,
        is_admin: bool,
        access_token: String,
        expires_at: Option<i64>,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            id,
            strava_id,
            firstname,
            name,
            is_admin,
            access_token,
            expires_at,
            refresh_token,
        }
    }

    async fn get_strava(
        &self,
        uri: &str,
        conn: &mut impl StravaStore,
    ) -> AnyResult<reqwest::Response> {
        let resp = reqwest::Client::new()
            .get(format!("{}{}", API, uri))
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Could not reach strava")?;

        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }

        match status {
            StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT => {
                bail!(Error::TryAgain(status.canonical_reason().unwrap()))
            }
            StatusCode::UNAUTHORIZED => {
                // self.disable(conn).await?;
                bail!(Error::NotAuth(
                    "Strava request authorization withdrawn".to_string()
                ))
            }
            _ => bail!(Error::BadRequest(format!(
                "Strava request error: {}",
                status
                    .canonical_reason()
                    .unwrap_or("Unknown status received")
            ))),
        }
    }
}

impl Person for RequestUser {
    fn get_id(&self) -> UserId {
        self.id
    }
    fn is_admin(&self) -> bool {
        self.is_admin
    }
}

#[async_trait]
impl StravaPerson for RequestUser {
    fn strava_id(&self) -> StravaId {
        self.strava_id
    }

    fn tb_id(&self) -> UserId {
        self.id
    }

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        conn: &mut impl StravaStore,
    ) -> AnyResult<T> {
        let r = self.get_strava(uri, conn).await?.text().await?;
        serde_json::from_str::<T>(&r).context("Could not parse response body")
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RequestUser
where
    MemoryStore: FromRef<S>,
    S: Send + Sync,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = AuthRedirect;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let store = MemoryStore::from_ref(state);

        let cookies = parts
            .extract::<TypedHeader<headers::Cookie>>()
            .await
            .map_err(|e| match *e.name() {
                header::COOKIE => match e.reason() {
                    TypedHeaderRejectionReason::Missing => AuthRedirect,
                    _ => panic!("unexpected error getting Cookie header(s): {}", e),
                },
                _ => panic!("unexpected error getting cookies: {}", e),
            })?;
        let session_cookie = cookies
            .get(crate::strava::COOKIE_NAME)
            .ok_or(AuthRedirect)?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .expect("could not load session")
            .ok_or(AuthRedirect)?;

        let user = session.get::<RequestUser>("user").ok_or(AuthRedirect)?;

        Ok(user)
    }
}

pub struct AxumAdmin;

#[async_trait]
impl<S> FromRequestParts<S> for AxumAdmin
where
    MemoryStore: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = RequestUser::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        if !user.is_admin() {
            Err(StatusCode::NOT_FOUND.into_response())
        } else {
            Ok(AxumAdmin)
        }
    }
}
