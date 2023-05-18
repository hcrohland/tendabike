//! Example OAuth (Strava) implementation.
//!
//! 1) Create client_id and client_secret at <https://www.strava.com/settings/api>
//! 2) Run with the following (replacing values appropriately):
//! ```not_rust
//! CLIENT_ID=REPLACE_ME CLIENT_SECRET=REPLACE_ME cargo run -p example-oauth
//! ```

use async_session::{MemoryStore, Session, SessionStore};
use axum::{
    async_trait,
    extract::{
        rejection::TypedHeaderRejectionReason, FromRef, FromRequestParts, Query, State, TypedHeader,
    },
    http::{header::SET_COOKIE, HeaderMap},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    RequestPartsExt, Router,
};
use http::{header, request::Parts};
use oauth2::{
    basic::{BasicErrorResponse, BasicTokenType, BasicTokenIntrospectionResponse, BasicRevocationErrorResponse}, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, RedirectUrl, Scope, TokenUrl, ExtraTokenFields, Client, StandardRevocableToken, StandardTokenResponse,
};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static COOKIE_NAME: &str = "SESSION";

#[tokio::main]
pub async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "oauth2,reqwest,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // `MemoryStore` is just used as an example. Don't use this in production.
    let store = MemoryStore::new();
    let oauth_client = oauth_client();
    let app_state = AppState {
        store,
        oauth_client,
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/auth/strava", get(strava_auth))
        .route("/auth/authorized", get(login_authorized))
        .route("/protected", get(protected))
        .route("/logout", get(logout))
        .route(
            "/body",
            post(|payload: String| async move {
                dbg!(payload)})
        )
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Clone)]
struct AppState {
    store: MemoryStore,
    oauth_client: StravaClient,
}

impl FromRef<AppState> for MemoryStore {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

impl FromRef<AppState> for StravaClient {
    fn from_ref(state: &AppState) -> Self {
        state.oauth_client.clone()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StravaAthleteInfo {
    id: i32,
    firstname: String,
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
struct StravaExtraTokenFields {
    athlete: StravaAthleteInfo
}

impl ExtraTokenFields for StravaExtraTokenFields {}

type StravaTokenResponse = StandardTokenResponse<StravaExtraTokenFields, BasicTokenType>;

type StravaClient = Client<BasicErrorResponse, StravaTokenResponse, BasicTokenType, BasicTokenIntrospectionResponse, StandardRevocableToken, BasicRevocationErrorResponse>;

fn oauth_client() -> StravaClient {
    // Environment variables (* = required):
    // *"CLIENT_ID"     "REPLACE_ME";
    // *"CLIENT_SECRET" "REPLACE_ME";
    //  "REDIRECT_URL"  "http://127.0.0.1:3000/auth/authorized";
    //  "AUTH_URL"      "https://www.strava.com/oauth/authorize";
    //  "TOKEN_URL"     "https://www.strava.com/oauth/token";

    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID!");
    let client_secret = env::var("CLIENT_SECRET").expect("Missing CLIENT_SECRET!");
    let redirect_url = env::var("REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:3000/auth/authorized".to_string());

    let auth_url = env::var("AUTH_URL").unwrap_or_else(|_| {
        "https://www.strava.com/oauth/authorize".to_string()
    });

    let token_url = env::var("TOKEN_URL")
        .unwrap_or_else(|_| "https://www.strava.com/oauth/token".to_string());

    StravaClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url).unwrap(),
        Some(TokenUrl::new(token_url).unwrap()),
    )
    .set_auth_type(oauth2::AuthType::RequestBody)
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}

// Session is optional
async fn index(user: Option<StravaAthleteInfo>) -> impl IntoResponse {
    match user {
        Some(u) => format!(
            "Hey {}! You're logged in!\nYou may now access `/protected`.\nLog out with `/logout`.",
            u.firstname
        ),
        None => "You're not logged in.\nVisit `/auth/strava` to do so.".to_string(),
    }
}

async fn strava_auth(State(client): State<StravaClient>) -> impl IntoResponse {
    let (auth_url, _csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        // .add_scope(Scope::new("activity:read_all,profile:read_all".to_string()))
        .add_scope(Scope::new("read,activity:read_all,profile:read_all".to_string()))
        .url();

    // Redirect to Strava's oauth service
    Redirect::to(auth_url.as_ref())
}

// Valid user session required. If there is none, redirect to the auth page
async fn protected(user: StravaAthleteInfo) -> impl IntoResponse {
    format!(
        "Welcome to the protected area :)\nHere's your info:\n{:?}",
        user
    )
}

async fn logout(
    State(store): State<MemoryStore>,
    TypedHeader(cookies): TypedHeader<headers::Cookie>,
) -> impl IntoResponse {
    let cookie = cookies.get(COOKIE_NAME).unwrap();
    let session = match store.load_session(cookie.to_string()).await.unwrap() {
        Some(s) => s,
        // No session active, just redirect
        None => return Redirect::to("/"),
    };

    store.destroy_session(session).await.unwrap();

    Redirect::to("/")
}

#[derive(Debug, Serialize,Deserialize)]
#[allow(dead_code)]
struct AuthRequest {
    code: String,
    state: String,
    scope: String,
}

async fn login_authorized(
    Query(query): Query<AuthRequest>,
    State(store): State<MemoryStore>,
    State(oauth_client): State<StravaClient>,
) -> impl IntoResponse {
    // Get an auth token
    dbg!(&query);

    let token = oauth_client
        .exchange_code(AuthorizationCode::new(query.code.clone()))
        .request_async(async_http_client)
        .await;


    if let Err(err) = token {
        if let oauth2::RequestTokenError::Parse(a,buf) = &err {
            dbg!(a);
            eprintln!("{}", String::from_utf8_lossy(&buf));
        } else {
            dbg!(&err);
        }
        return (http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response()
    }

    let token= token.unwrap();

    dbg!(&token);
    
    // Create a new session filled with user data
    let mut session = Session::new();
    session.insert("user", &token.extra_fields().athlete).unwrap();

    // Store session and get corresponding cookie
    let cookie = store.store_session(session).await.unwrap().unwrap();

    // Build the cookie
    let cookie = format!("{}={}; SameSite=Lax; Path=/", COOKIE_NAME, cookie);

    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (headers, Redirect::to("/")).into_response()
}

struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        Redirect::temporary("/auth/strava").into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for StravaAthleteInfo
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
        let session_cookie = cookies.get(COOKIE_NAME).ok_or(AuthRedirect)?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .unwrap()
            .ok_or(AuthRedirect)?;

        let user = session.get::<StravaAthleteInfo>("user").ok_or(AuthRedirect)?;

        Ok(user)
    }
}