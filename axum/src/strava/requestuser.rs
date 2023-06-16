use std::time::SystemTime;

use anyhow::Context;
use async_session::{
    async_trait,
    log::{debug, trace},
    MemoryStore, SessionStore,
};
use axum::{
    extract::{rejection::TypedHeaderRejectionReason, FromRef, FromRequestParts},
    response::{IntoResponse, Response},
    RequestPartsExt, TypedHeader,
};
use http::{header, request::Parts, StatusCode};
use oauth2::{
    basic::BasicTokenType, reqwest::async_http_client, AccessToken, RefreshToken,
    StandardTokenResponse, TokenResponse,
};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use tb_domain::{Error, Person, TbResult, UserId};
use tb_strava::{StravaId, StravaPerson, StravaStore, StravaUser};

use crate::{
    error::AuthRedirect,
    strava::{oauth::STRAVACLIENT, StravaAthleteInfo},
};

use super::StravaExtraTokenFields;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RequestUser {
    id: UserId,
    strava_id: StravaId,
    is_admin: bool,
    access_token: AccessToken,
    expires_at: Option<SystemTime>,
    refresh_token: Option<RefreshToken>,
}

const API: &str = "https://www.strava.com/api/v3";

impl RequestUser {
    pub(crate) async fn create_from_token(
        token: StandardTokenResponse<StravaExtraTokenFields, BasicTokenType>,
        conn: &mut impl StravaStore,
    ) -> TbResult<Self> {
        trace!("got token {:?})", &token);

        let StravaAthleteInfo {
            id,
            firstname,
            lastname,
            ..
        } = token
            .extra_fields()
            .athlete
            .as_ref()
            .ok_or(Error::BadRequest("No Athlete Info from Strava".to_string()))?;

        let refresh_token = token.refresh_token();
        let refresh = refresh_token.map(|t| t.secret());
        let user = StravaUser::upsert(*id, &firstname, &lastname, refresh, conn).await?;
        let id = user.tb_id();
        let is_admin = id.is_admin(conn).await?;

        Ok(Self {
            id,
            strava_id: user.strava_id(),
            is_admin,
            access_token: token.access_token().clone(),
            expires_at: token.expires_in().map(|d| SystemTime::now() + d),
            refresh_token: refresh_token.cloned(),
        })
    }

    pub(crate) async fn create_from_id(
        _admin: AxumAdmin,
        user: UserId,
        conn: &mut impl StravaStore,
    ) -> TbResult<RequestUser> {
        let user = StravaUser::read(user, conn).await?;

        let strava_id = user.strava_id();
        let id = user.tb_id();
        let is_admin = id.is_admin(conn).await?;
        let refresh_token = user.refresh_token().ok_or(Error::BadRequest(format!(
            "User {} does not have a refresh token (disabled?)",
            id
        )))?;

        Ok(Self {
            id,
            strava_id,
            is_admin,
            access_token: AccessToken::new(String::default()),
            expires_at: Some(SystemTime::UNIX_EPOCH),
            refresh_token: Some(RefreshToken::new(refresh_token)),
        })
    }

    fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => SystemTime::now() > expires_at,
            None => false,
        }
    }

    async fn refresh_the_token(
        &mut self,
        conn: &mut impl StravaStore,
    ) -> TbResult<()> {
        let token = match self.refresh_token.clone() {
            Some(token) => token,
            None => {
                return Err(Error::NotAuth(
                    "No Refresh Token provided and Access Token expired".to_string(),
                ))
            }
        };
        debug!("refreshing token for user {}", self.id);
        let token = match STRAVACLIENT
            .exchange_refresh_token(&token)
            .request_async(async_http_client)
            .await {
                Ok(token) => token,
                Err(err) => return Err(Error::NotAuth(err.to_string()))
        };
        self.access_token = token.access_token().clone();
        self.expires_at = token.expires_in().map(|d| SystemTime::now() + d);
        self.refresh_token = token.refresh_token().cloned();
        let refresh = token.refresh_token().map(|t| t.secret());
        self.strava_id.update_token(refresh, conn).await?;
        Ok(())
    }

    async fn get_strava(
        &mut self,
        uri: &str,
        conn: &mut impl StravaStore,
    ) -> TbResult<reqwest::Response> {
        self.check_token(conn).await?;

        let resp = reqwest::Client::new()
            .get(format!("{}{}", API, uri))
            .bearer_auth(&self.access_token.secret())
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
                return Err(Error::TryAgain(status.canonical_reason().unwrap()))
            }
            StatusCode::UNAUTHORIZED => {
                return Err(Error::NotAuth(format!(
                    "Strava request authorization withdrawn for {}",
                    uri
                )));
            }
            _ => {
                return Err(Error::BadRequest(format!(
                    "Strava request error: {}",
                    status
                        .canonical_reason()
                        .unwrap_or("Unknown status received")
                )))
            }
        }
    }

    async fn check_token(&mut self, conn: &mut impl StravaStore) -> TbResult<()> {
        Ok(if self.is_expired() {
            debug!("access token for user {} is expired", self.id);
            self.refresh_the_token(conn).await?
        })
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

    async fn request_json<T: DeserializeOwned>(
        &mut self,
        uri: &str,
        conn: &mut impl StravaStore,
    ) -> TbResult<T> {
        let r = self
            .get_strava(uri, conn)
            .await?
            .text()
            .await
            .context("could not reach strava")?;
        Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
    }

    async fn deauthorize(&mut self, conn: &mut impl StravaStore) -> TbResult<()> {
        self.check_token(conn).await?;
        STRAVACLIENT
            .revoke_token(self.access_token.clone().into())
            .context("revoke token config error")?
            .add_extra_param("access_token", self.access_token.secret()) // Strava does not follow the standard.
            .request_async(async_http_client)
            .await
            .context("token exchange failed")?;
        debug!("user {} token revoked", self.strava_id);
        Ok(())
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
            Err((StatusCode::NOT_FOUND, "Page not found").into_response())
        } else {
            Ok(AxumAdmin)
        }
    }
}
