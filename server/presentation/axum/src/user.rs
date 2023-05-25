use async_session::{async_trait, MemoryStore, SessionStore};
use axum::{
    extract::{rejection::TypedHeaderRejectionReason, FromRef, FromRequestParts},
    RequestPartsExt, TypedHeader, response::{Response, IntoResponse}, Router, routing::get, Json,
};
use http::{header, request::Parts, StatusCode};
use kernel::domain::{Person, UserId};
use serde_derive::{Deserialize, Serialize};

use crate::{strava::AuthRedirect, AppError, AppDbConn};

pub(crate) fn router(state: crate::AppState) -> Router{
    Router::new()
        .route("/", get(getuser))
        // .route("/summary", get(summary))
        .route("/all", get(userlist))
        .with_state(state)
}

async fn getuser(user: RUser) -> Json<RUser> {
    Json(user)
}

type ApiResult<T> = Result<Json<T>, AppError>;

// async fn summary(user: strava::User, mut conn: AppDbConn) -> ApiResult<Summary> {
//     user.get_summary(&mut conn).map(Json)
// }

async fn userlist(_u: AxumAdmin, mut pool: AppDbConn) -> ApiResult<Vec<tb_strava::StravaStat>> {
    Ok(tb_strava::get_all_stats(&mut pool).map(Json)?)
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RUser { 
    pub user: UserId,
    pub firstname: String,
    pub name: String,
    pub is_admin: bool
}

impl RUser {
    pub(crate) fn new(user: UserId, firstname: String, name: String, is_admin: bool) -> Self { Self { user, firstname, name, is_admin } }
}

impl Person for RUser {
    fn get_id(&self) -> UserId {
        self.user
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
            .unwrap()
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