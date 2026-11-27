//! This module contains the implementation of the StravaUser struct and its methods.
//!
//! The StravaUser struct represents a user of the Strava API and contains information such as the user's
//! Strava ID, Tendabike ID, access token, and refresh token.
//!
//! The methods implemented for the StravaUser struct allow for reading and updating user data, as well as
//! checking the validity of the user's access token.

use anyhow::Context;
use derive_more::{Display, From, Into};
use log::{debug, info};
use oauth2::RefreshToken;
use serde::{Deserialize, Serialize};

use super::{Error, StravaSpec, TbResult, UserId};
use crate::EntityFacade;
use crate::store::Store;

/// The Strava athlete/user id.
#[derive(
    Clone, Copy, Debug, Display, From, Into, Default, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct StravaId(i64);

impl StravaId {
    pub(crate) async fn read(
        &self,
        store: &mut impl Store<StravaSpec>,
    ) -> TbResult<Option<StravaUser>> {
        store.user_get_by_provider_id(self).await
    }

    /// update the refresh token for the user
    ///
    /// sets a five minute buffer for the access token
    /// returns the updated user
    #[cfg(test)]
    pub(crate) async fn update_token(
        self,
        refresh: Option<String>,
        store: &mut impl Store<StravaSpec>,
    ) -> TbResult<StravaUser> {
        store.user_update_token(self, refresh).await
    }

    /// disable a user
    pub async fn disable(self, store: &mut impl Store<StravaSpec>) -> TbResult<()> {
        info!("disabling user {self}");
        store.user_disable(self).await
    }
}

/// Strava User data
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct StravaUser {
    /// the Strava user id
    pub id: StravaId,
    /// the corresponding tendabike user id
    pub tendabike_id: UserId,
    /// the refresh token to get a new access token from Strava
    pub refresh_token: Option<RefreshToken>,
}

impl StravaUser {
    /// Reads the StravaUser data for the given `id` from the database.
    ///
    /// # Arguments
    ///
    /// * `id` - A `UserId` representing the Tendabike user ID.
    /// * `store` - A mutable reference to the database connection.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the user is not registered.
    #[cfg(test)]
    pub(crate) async fn read(id: UserId, store: &mut impl Store<StravaSpec>) -> TbResult<Self> {
        store.user_get_by_tbid(id).await
    }

    /// get the tendabike id for this user
    pub fn user_id(&self) -> UserId {
        self.tendabike_id
    }

    /// get the strava id for this user
    pub(crate) fn strava_id(&self) -> StravaId {
        self.id
    }

    pub(crate) fn refresh_token(&self) -> Option<RefreshToken> {
        self.refresh_token.clone()
    }

    pub(crate) fn disabled(&self) -> bool {
        self.refresh_token.is_none()
    }

    /// Upsert a Strava user by ID, updating their Tendabike user ID if they already exist, or creating a new user if they don't.
    ///
    /// # Arguments
    ///
    /// * `id` - A `StravaId` representing the ID of the Strava user to upsert.
    /// * `firstname` - A `&str` representing the first name of the Strava user.
    /// * `lastname` - A `&str` representing the last name of the Strava user.
    /// * `store` - A mutable reference to a `AppConn` representing the database connection.
    ///
    /// # Returns
    ///
    /// An `TbResult` containing a `StravaUser` representing the upserted user.
    pub async fn upsert<D: EntityFacade<S>, S: Store<StravaSpec>>(
        id: StravaId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        refresh: Option<&RefreshToken>,
        facade: &D,
        store: &mut S,
    ) -> TbResult<StravaUser> {
        debug!("got id {}: {} {}", id, firstname, lastname);

        let user = id.read(store).await?;
        if let Some(user) = user {
            facade
                .update_user(&user.tendabike_id, firstname, lastname, avatar, store)
                .await?;
            store
                .user_update_token(user.id, refresh.map(|t| t.secret().to_string()))
                .await?;
            return Ok(user);
        }

        // create new user!
        let tendabike_id = facade
            .create_user(firstname, lastname, avatar, store)
            .await?;

        let user = StravaUser {
            id,
            tendabike_id,
            refresh_token: refresh.cloned(),
        };
        info!("creating new user id {user:?}");

        let user = store.user_create(user).await?;
        // Note: Automatic sync removed - user must manually trigger initial activity import
        Ok(user)
    }
}

/// The login (ADR-0014): the provider's login work behind the seam — the
/// scope check, the token exchange, the wire-shape parsing of the athlete
/// (plus the avatar normalization: a non-`http` avatar is dropped), and the
/// upsert, returning the identity and the admin flag.
pub(crate) async fn do_login<D: EntityFacade<S>, S: Store<StravaSpec>>(
    code: &str,
    scope: &str,
    facade: &D,
    store: &mut S,
) -> TbResult<(UserId, bool)> {
    use super::StravaAthleteInfo;
    use super::client::{SCOPES, strava_client};
    use crate::client::HTTP_CLIENT;
    use oauth2::{AuthorizationCode, TokenResponse};

    if scope != SCOPES {
        return Err(Error::BadRequest(format!(
            "insufficient authorization {scope}"
        )));
    }

    let token = strava_client()
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .request_async(&*HTTP_CLIENT)
        .await
        .context("token exchange failed")?;

    let mut athlete: StravaAthleteInfo = token
        .extra_fields()
        .athlete
        .clone()
        .ok_or(Error::BadRequest("No Athlete Info from Strava".to_string()))?;
    // make sure you have a non-local url
    if let Some(a) = &athlete.avatar
        && !a.starts_with("http")
    {
        athlete.avatar = None;
    }
    let refresh_token = token.refresh_token().cloned();

    let user = StravaUser::upsert(
        athlete.id,
        &athlete.firstname,
        &athlete.lastname,
        &athlete.avatar,
        refresh_token.as_ref(),
        facade,
        store,
    )
    .await?;
    let is_admin = facade.user_admin(&user.user_id(), store).await?;
    Ok((user.user_id(), is_admin))
}

#[cfg(test)]
mod user_tests;
