//! This module contains the implementation of the IcuUser struct and its methods.
//!
//! The IcuUser struct represents a user of the intervals.icu API and contains
//! the user's icu athlete ID, their TendaBike user ID, and their (durable)
//! access token. intervals.icu tokens do not expire and are never refreshed,
//! so there is no refresh-token machinery (unlike the Strava driver).
//!
//! The `upsert` op is the login upsert (spec #763): one transaction that
//! creates the domain user (with `provider = 'icu'`) plus the `icu_users`
//! row, or updates both for an existing connection.

use derive_more::{Display, From, Into};
use serde::Deserialize;

use super::*;
use crate::DomainFacade;

/// The icu athlete ID: a provider text id (intervals.icu ids are not
/// integers), wrapped as an opaque newtype. The inner value is private —
/// compare/convert with `String::from(id)` / `IcuId::from(s)`.
#[derive(
    Clone, Debug, From, Into, PartialEq, Eq, Hash, Default, Display, Serialize, Deserialize,
)]
#[display("{}", _0)]
pub struct IcuId(String);

impl From<&str> for IcuId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl IcuId {
    /// Read the icu user for this athlete id, if one exists.
    pub(crate) async fn read(&self, store: &mut impl IcuDataStore) -> TbResult<Option<IcuUser>> {
        store.icuser_get_by_icuid(self).await
    }

    /// Disable the user (a 401 on a live request, spec #764): drop their
    /// queued events and clear their (durable) access token. The cleared
    /// token is the disable signal — [`IcuUser::disabled`] reports it.
    pub(crate) async fn disable(&self, store: &mut impl IcuDataStore) -> TbResult<()> {
        info!("disabling icu user {self}");
        let events = store.icu_events_delete_for_user(self).await?;
        if events > 0 {
            info!("deleted {events} open events for icu user {self}");
        }
        store.icuser_update_token(self.clone(), None).await?;
        Ok(())
    }
}

/// Icu User data.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct IcuUser {
    /// The icu athlete id.
    pub id: IcuId,
    /// The corresponding tendabike user id.
    pub tendabike_id: UserId,
    /// The durable access token (intervals.icu tokens never refresh).
    pub access_token: Option<String>,
    /// Whether the user's icu integration is disabled.
    pub disabled: bool,
}

impl IcuUser {
    /// Get the tendabike id for this user.
    pub fn user_id(&self) -> UserId {
        self.tendabike_id
    }

    /// Whether the user's icu integration is disabled: the (durable) access
    /// token is cleared (a 401's disable, spec #764).
    pub(crate) fn disabled(&self) -> bool {
        self.access_token.is_none()
    }

    /// Upsert an icu user by athlete ID, updating their TendaBike user if
    /// they already exist, or creating a new user (with `provider = 'icu'`)
    /// if they don't (spec #763).
    ///
    /// # Arguments
    ///
    /// * `id` - A `IcuId` representing the icu athlete ID to upsert.
    /// * `name` - A `&str` representing the athlete's full name.
    /// * `access_token` - A `&str` representing the durable access token.
    /// * `facade` - The domain door.
    /// * `store` - A mutable reference to the icu data store.
    ///
    /// # Returns
    ///
    /// An `TbResult` containing the upserted [`IcuUser`].
    pub async fn upsert<D: DomainFacade<S>, S: IcuDataStore>(
        id: IcuId,
        name: &str,
        access_token: &str,
        facade: &D,
        store: &mut S,
    ) -> TbResult<IcuUser> {
        let (firstname, lastname) = name.split_once(' ').unwrap_or((name, ""));
        let avatar = None;
        debug!("got id {id}: {name}");

        let user = id.read(store).await?;
        if let Some(user) = user {
            facade
                .update_user(&user.tendabike_id, firstname, lastname, &avatar, store)
                .await?;
            store
                .icuser_update_token(user.id.clone(), Some(access_token.to_string()))
                .await?;
            return Ok(user);
        }

        // create new user!
        let tendabike_id = facade
            .create_user(firstname, lastname, &avatar, "icu", store)
            .await?;

        let user = IcuUser {
            id,
            tendabike_id,
            access_token: Some(access_token.to_string()),
            disabled: false,
        };
        info!("creating new icu user id {user:?}");

        let user = store.icuser_create(user).await?;
        Ok(user)
    }
}

#[cfg(test)]
mod user_tests;
