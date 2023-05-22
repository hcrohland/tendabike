use async_session::{async_trait, MemoryStore, SessionStore};
use axum::{
    extract::{rejection::TypedHeaderRejectionReason, FromRef, FromRequestParts},
    RequestPartsExt, TypedHeader, response::{Response, IntoResponse},
};
use http::{header, request::Parts, StatusCode};
use kernel::domain::{Person, UserId};
use serde_derive::{Deserialize, Serialize};

use crate::oauth::AuthRedirect;
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RUser { 
    pub user: UserId,
    pub firstname: String,
    pub lastname: String,
    pub is_admin: bool
}

impl RUser {
    pub(crate) fn new(user: UserId, firstname: String, lastname: String, is_admin: bool) -> Self { Self { user, firstname, lastname, is_admin } }
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
        let session_cookie = cookies.get(crate::oauth::COOKIE_NAME).ok_or(AuthRedirect)?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .unwrap()
            .ok_or(AuthRedirect)?;

        let user = session.get::<RUser>("user").ok_or(AuthRedirect)?;

        Ok(user)
    }
}

pub struct Admin;

#[async_trait]
impl<S> FromRequestParts<S> for Admin 
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
            Ok(Admin)
        }
    }
}