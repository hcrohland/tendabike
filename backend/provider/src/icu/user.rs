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

    /// Upsert an icu user by athlete ID, updating their TendaBike user if
    /// they already exist, or creating a new user (with `provider = 'icu'`)
    /// if they don't (spec #763).
    ///
    /// # Arguments
    ///
    /// * `id` - A `IcuId` representing the icu athlete ID to upsert.
    /// * `firstname` - A `&str` representing the first name.
    /// * `lastname` - A `&str` representing the last name.
    /// * `avatar` - An `&Option<String>` representing the avatar URL.
    /// * `access_token` - A `&str` representing the durable access token.
    /// * `facade` - The domain door.
    /// * `store` - A mutable reference to the icu data store.
    ///
    /// # Returns
    ///
    /// An `TbResult` containing the upserted [`IcuUser`].
    pub async fn upsert<D: DomainFacade<S>, S: IcuDataStore>(
        id: IcuId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        access_token: &str,
        facade: &D,
        store: &mut S,
    ) -> TbResult<IcuUser> {
        debug!("got id {id}: {firstname} {lastname}");

        let user = id.read(store).await?;
        if let Some(user) = user {
            facade
                .update_user(&user.tendabike_id, firstname, lastname, avatar, store)
                .await?;
            store
                .icuser_update_token(user.id.clone(), Some(access_token.to_string()))
                .await?;
            return Ok(user);
        }

        // create new user!
        let tendabike_id = facade
            .create_user(firstname, lastname, avatar, "icu", store)
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

/// The [`ActivityId`] for an icu activity, rendered as `"icu:<id>"`.
///
/// The `icu:` prefix is an icu concern: the domain's `ActivityId` is an
/// opaque string (ADR-0013), so the prefix construction lives here, private
/// to the icu module. Its consumer is the activity-import op (a later
/// ticket, spec #763); it is kept now so the prefix is fixed and tested.
#[allow(dead_code)]
fn activity_id(id: &str) -> ActivityId {
    ActivityId::from(format!("icu:{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icu_activity_id_uses_icu_prefix() {
        assert_eq!(activity_id("42"), ActivityId::from("icu:42"));
    }
}

#[cfg(test)]
mod user_tests;
