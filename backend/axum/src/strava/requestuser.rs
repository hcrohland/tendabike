use anyhow::Context;
use async_session::{
    MemoryStore, Session, SessionStore, async_trait,
    log::{debug, trace},
};
use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts},
    response::{IntoResponse, Response},
};
use axum_extra::TypedHeader;
use http::{StatusCode, request::Parts};
use oauth2::{
    AccessToken, RefreshToken, StandardTokenResponse, TokenResponse, basic::BasicTokenType, reqwest,
};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use std::time::SystemTime;

use super::StravaExtraTokenFields;
use crate::{
    AppError,
    strava::{HTTP_CLIENT, StravaAthleteInfo, oauth::STRAVACLIENT},
};
use tb_domain::{Error, Person, TbResult, UserId};
use tb_strava::{StravaId, StravaPerson, StravaStore, StravaUser};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RequestUser {
    id: UserId,
    strava_id: StravaId,
    is_admin: bool,
    access_token: AccessToken,
    expires_at: Option<SystemTime>,
    refresh_token: Option<RefreshToken>,
    #[serde(skip)]
    session: Option<Session>,
}

const API: &str = "https://www.strava.com/api/v3";

impl RequestUser {
    pub(crate) async fn create_from_token(
        token: StandardTokenResponse<StravaExtraTokenFields, BasicTokenType>,
        store: &mut impl StravaStore,
    ) -> TbResult<Self> {
        trace!("got token {:?})", &token);

        let StravaAthleteInfo {
            id,
            firstname,
            lastname,
            avatar,
            ..
        } = token
            .extra_fields()
            .athlete
            .as_ref()
            .ok_or(Error::BadRequest("No Athlete Info from Strava".to_string()))?;

        // make sure you have a non-local url
        let avatar = match avatar {
            Some(a) if !a.starts_with("http") => &None,
            _ => avatar,
        };
        let refresh_token = token.refresh_token();
        let refresh = refresh_token.map(|t| t.secret());
        let user = StravaUser::upsert(*id, firstname, lastname, avatar, refresh, store).await?;
        let id = user.tb_id();
        let is_admin = id.is_admin(store).await?;

        Ok(Self {
            id,
            strava_id: user.strava_id(),
            is_admin,
            access_token: token.access_token().clone(),
            expires_at: token.expires_in().map(|d| SystemTime::now() + d),
            refresh_token: refresh_token.cloned(),
            session: None,
        })
    }

    pub(crate) async fn create_from_id(
        _admin: AxumAdmin,
        user: UserId,
        store: &mut impl StravaStore,
    ) -> TbResult<RequestUser> {
        let user = StravaUser::read(user, store).await?;

        let strava_id = user.strava_id();
        let id = user.tb_id();
        let is_admin = id.is_admin(store).await?;
        let refresh_token = user.refresh_token().ok_or(Error::BadRequest(format!(
            "User {id} does not have a refresh token (disabled?)"
        )))?;

        Ok(Self {
            id,
            strava_id,
            is_admin,
            access_token: AccessToken::new(String::default()),
            expires_at: Some(SystemTime::UNIX_EPOCH),
            refresh_token: Some(RefreshToken::new(refresh_token)),
            session: None,
        })
    }

    fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires_at) => SystemTime::now() > expires_at,
            None => false,
        }
    }

    async fn refresh_the_token(&mut self, store: &mut impl StravaStore) -> TbResult<()> {
        let token = match self.refresh_token.clone() {
            Some(token) => token,
            None => {
                return Err(Error::NotAuth(
                    "No Refresh Token provided and Access Token expired".to_string(),
                ));
            }
        };
        debug!("refreshing token for user {}", self.id);
        let token = match STRAVACLIENT
            .exchange_refresh_token(&token)
            .request_async(&*HTTP_CLIENT)
            .await
        {
            Ok(token) => token,
            Err(err) => return Err(Error::NotAuth(err.to_string())),
        };
        self.access_token = token.access_token().clone();
        self.expires_at = token.expires_in().map(|d| SystemTime::now() + d);
        self.refresh_token = token.refresh_token().cloned();
        let refresh = token.refresh_token().map(|t| t.secret());
        self.strava_id.update_token(refresh, store).await?;
        if let Some(mut session) = self.session.clone() {
            debug!("updating session for user {}", self.id);
            session
                .insert("user", self)
                .context("session insert failed")?;
        };
        Ok(())
    }

    async fn get_strava(
        &mut self,
        uri: &str,
        store: &mut impl StravaStore,
    ) -> TbResult<reqwest::Response> {
        self.check_token(store).await?;

        let resp = reqwest::Client::new()
            .get(format!("{API}{uri}"))
            .bearer_auth(self.access_token.secret())
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
                Err(Error::TryAgain(status.canonical_reason().unwrap()))
            }
            StatusCode::UNAUTHORIZED => Err(Error::NotAuth(format!(
                "Strava request authorization withdrawn for {uri}"
            ))),
            _ => Err(Error::BadRequest(format!(
                "Strava request error: {}",
                status
                    .canonical_reason()
                    .unwrap_or("Unknown status received")
            ))),
        }
    }

    async fn check_token(&mut self, store: &mut impl StravaStore) -> TbResult<()> {
        if self.is_expired() {
            debug!("access token for user {} is expired", self.id);
            return self.refresh_the_token(store).await;
        }
        Ok(())
    }

    fn set_session(mut self, session: Session) -> Self {
        self.session = Some(session);
        self
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
        store: &mut impl StravaStore,
    ) -> TbResult<T> {
        let r = self
            .get_strava(uri, store)
            .await?
            .text()
            .await
            .context("could not reach strava")?;
        trace!("{uri}:\n{r}");
        Ok(serde_json::from_str::<T>(&r).context("Could not parse response body")?)
    }

    async fn deauthorize(&mut self, store: &mut impl StravaStore) -> TbResult<()> {
        self.check_token(store).await?;

        STRAVACLIENT
            .revoke_token(self.access_token.clone().into())
            .context("revoke token config error")?
            .add_extra_param("access_token", self.access_token.secret()) // Strava does not follow the standard.
            .request_async(&*HTTP_CLIENT)
            .await
            .context("token exchange failed")?;
        debug!("user {} token revoked", self.strava_id);
        Ok(())
    }
}

impl<S> FromRequestParts<S> for RequestUser
where
    MemoryStore: FromRef<S>,
    S: Send + Sync,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let store = MemoryStore::from_ref(state);

        let cookies = parts
            .extract::<TypedHeader<headers::Cookie>>()
            .await
            .map_err(anyhow::Error::new)?;
        let session_cookie = cookies
            .get(crate::strava::COOKIE_NAME)
            .ok_or(Error::NotAuth("Please authenticate".into()))?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .expect("could not load session")
            .ok_or(Error::NotAuth("Please authenticate".into()))?;

        let user = session
            .get::<RequestUser>("user")
            .ok_or(Error::NotAuth("Session error".into()))?
            .set_session(session);

        Ok(user)
    }
}

pub struct AxumAdmin;

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
            Err((http::StatusCode::NOT_FOUND, "Page not found").into_response())
        } else {
            Ok(AxumAdmin)
        }
    }
}
