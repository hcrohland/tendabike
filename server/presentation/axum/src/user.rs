//! This module contains the implementation of user-related routes and handlers for the Axum web framework.
//!
//! The routes in this module are used to retrieve user information, summaries, and lists of users.
//! The handlers in this module interact with the database and Strava API to retrieve and process user data.
//!
//! This module also defines the `RUser` struct, which represents a user in the system and is used throughout the module.
//! Additionally, it defines the `AxumAdmin` struct, which is used as a marker type for routes that require admin privileges.

use async_session::{async_trait, MemoryStore, SessionStore};
use axum::{
    extract::{rejection::TypedHeaderRejectionReason, FromRef, FromRequestParts, State},
    RequestPartsExt, TypedHeader, response::{Response, IntoResponse}, Router, routing::get, Json,
};
use http::{header, request::Parts, StatusCode};
use kernel::{domain::{Person, UserId, Summary, AnyResult}, s_diesel::AppConn};
use serde_derive::{Deserialize, Serialize};
use tb_strava::{StravaId, StravaUser};

use crate::{AuthRedirect, ApiResult, appstate::AppState, DbPool};

pub(crate) fn router() -> Router<AppState>{
    Router::new()
        .route("/", get(getuser))
        .route("/summary", get(summary))
        .route("/all", get(userlist))
}

async fn getuser(user: RUser) -> Json<RUser> {
    Json(user)
}

async fn summary(user: RUser, State(conn): State<DbPool>) -> ApiResult<Summary> {
    let mut conn = conn.get().await?;
    Ok(user.get_strava_user(&mut conn).await?.get_summary(&mut conn).await.map(Json)?)
}

async fn userlist(_u: AxumAdmin, State(conn): State<DbPool>) -> ApiResult<Vec<tb_strava::StravaStat>> {
    let mut conn = conn.get().await?;
    Ok(tb_strava::get_all_stats(&mut conn).await.map(Json)?)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RUser { 
    pub id: UserId,
    pub strava_id: StravaId,
    pub firstname: String,
    pub name: String,
    pub is_admin: bool
}

impl RUser {
    pub(crate) fn new(id: UserId, strava_id: StravaId, firstname: String, name: String, is_admin: bool) -> Self { 
        Self { id, strava_id, firstname, name, is_admin } 
    }

    pub(crate) async fn get_strava_user(&self,  conn: &mut AppConn) -> AnyResult<StravaUser> {
        StravaUser::read(self.id, conn).await
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
        let session_cookie = cookies.get(crate::strava::COOKIE_NAME).ok_or(AuthRedirect)?;

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

    async fn from_request_parts(parts: &mut Parts, state: & S) ->  Result<Self,Self::Rejection> {
        let user = RUser::from_request_parts(parts, state).await.map_err(IntoResponse::into_response)?;
        if !user.is_admin() {
            Err(StatusCode::NOT_FOUND.into_response())
        } else {
            Ok(AxumAdmin)
        }
    }
}