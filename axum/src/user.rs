//! This module contains the implementation of user-related routes and handlers for the Axum web framework.
//!
//! The routes in this module are used to retrieve user information, summaries, and lists of users.
//! The handlers in this module interact with the database and Strava API to retrieve and process user data.
//!
//! This module also defines the `RUser` struct, which represents a user in the system and is used throughout the module.
//! Additionally, it defines the `AxumAdmin` struct, which is used as a marker type for routes that require admin privileges.

use anyhow::{Context, bail};
use async_session::{async_trait, MemoryStore, SessionStore};
use axum::{
    extract::{rejection::TypedHeaderRejectionReason, FromRef, FromRequestParts, State},
    response::{IntoResponse, Response},
    routing::get,
    Json, RequestPartsExt, Router, TypedHeader,
};
use http::{header, request::Parts, StatusCode};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use tb_domain::{AnyResult, Person, Summary, UserId, Error};
use tb_strava::{StravaId, StravaPerson, StravaStore, StravaUser};

use crate::{appstate::AppState, ApiResult, AuthRedirect, DbPool};

const API: &str = "https://www.strava.com/api/v3";

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(getuser))
        .route("/summary", get(summary))
        .route("/all", get(userlist))
}

async fn getuser(user: RUser) -> Json<RUser> {
    Json(user)
}

async fn summary(user: RUser, State(pool): State<DbPool>) -> ApiResult<Summary> {
    let mut conn = pool.get().await?;
    StravaUser::update_user(&user, &mut conn).await?;
    Ok(user.id.get_summary(&mut conn).await.map(Json)?)
}

async fn userlist(
    _u: AxumAdmin,
    State(pool): State<DbPool>,
) -> ApiResult<Vec<tb_strava::StravaStat>> {
    let mut conn = pool.get().await?;
    Ok(tb_strava::get_all_stats(&mut conn).await.map(Json)?)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RUser {
    id: UserId,
    strava_id: StravaId,
    firstname: String,
    name: String,
    is_admin: bool,
    access_token: String,
    expires_at: Option<i64>,
    refresh_token: Option<String>,
}

impl RUser {
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

    pub(crate) async fn get_strava_user(
        &self,
        conn: &mut impl StravaStore,
    ) -> AnyResult<StravaUser> {
        StravaUser::read(self.id, conn).await
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

impl Person for RUser {
    fn get_id(&self) -> UserId {
        self.id
    }
    fn is_admin(&self) -> bool {
        self.is_admin
    }
}

#[async_trait]
impl StravaPerson for RUser {
    fn strava_id(&self) -> StravaId {
        self.strava_id
    }

    fn tb_id(&self) -> UserId {
        self.id
    }

    async fn request_json<T: DeserializeOwned>(
        &self,
        uri: &str,
        conn: &mut impl StravaStore,
    ) -> AnyResult<T> {
        let r = self.get_strava(uri, conn).await?.text().await?;
        serde_json::from_str::<T>(&r).context("Could not parse response body")
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for RUser
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

        let user = session.get::<RUser>("user").ok_or(AuthRedirect)?;

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
        let user = RUser::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        if !user.is_admin() {
            Err(StatusCode::NOT_FOUND.into_response())
        } else {
            Ok(AxumAdmin)
        }
    }
}
