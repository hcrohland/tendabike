//! This module contains the implementation of the StravaUser struct and its methods.
//!
//! The StravaUser struct represents a user of the Strava API and contains information such as the user's
//! Strava ID, Tendabike ID, access token, and refresh token.
//!
//! The methods implemented for the StravaUser struct allow for reading and updating user data, as well as
//! checking the validity of the user's access token.

use newtype_derive::*;
use serde::Deserialize;

use crate::*;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct StravaId(i32);
NewtypeDisplay! { () pub struct StravaId(); }
NewtypeFrom! { () pub struct StravaId(i32); }

impl StravaId {
    pub async fn read(&self, store: &mut impl StravaStore) -> TbResult<Option<StravaUser>> {
        store.stravauser_get_by_stravaid(self).await
    }
}

/// Strava User data
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct StravaUser {
    /// the Strava user id
    pub id: StravaId,
    /// the corresponding tendabike user id
    pub tendabike_id: UserId,
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
    pub async fn read(id: UserId, store: &mut impl StravaStore) -> TbResult<Self> {
        store.stravauser_get_by_tbid(id).await
    }

    /// read the current user data for id
    /// get the tendabike id for this user
    pub fn tb_id(&self) -> UserId {
        self.tendabike_id
    }

    /// get the strava id for this user
    pub fn strava_id(&self) -> StravaId {
        self.id
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
    pub async fn upsert(
        id: StravaId,
        firstname: &str,
        lastname: &str,
        avatar: &Option<String>,
        store: &mut impl StravaStore,
    ) -> TbResult<StravaUser> {
        debug!("got id {}: {} {}", id, &firstname, &lastname);

        let user = id.read(store).await?;
        if let Some(user) = user {
            user.tendabike_id
                .update(firstname, lastname, avatar, store)
                .await?;
            return Ok(user);
        }

        // create new user!
        let tendabike_id = crate::UserId::create(firstname, lastname, avatar, store).await?;

        let user = StravaUser { id, tendabike_id };
        info!("creating new user id {user:?}");

        let user = store.stravauser_new(user).await?;
        event::insert_sync(user.id, 0, false, store).await?;
        Ok(user)
    }

    /// Get list of gear for user from Strava
    pub async fn update_gear(
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Vec<PartId>> {
        #[derive(Deserialize, Debug)]
        struct Gear {
            id: String,
        }

        #[derive(Deserialize, Debug)]
        struct Athlete {
            // firstname: String,
            // lastname: String,
            bikes: Vec<Gear>,
            shoes: Vec<Gear>,
        }

        let ath: Athlete = user.request_json("/athlete").await?;

        let mut parts = Vec::new();
        for gear in ath.bikes.into_iter().chain(ath.shoes) {
            parts.push(gear::into_partid(gear.id, user, store).await?);
        }

        Ok(parts)
    }

    pub async fn process(
        user: &mut impl StravaPerson,
        store: &mut impl StravaStore,
    ) -> TbResult<Summary> {
        event::process(user, store).await
    }
}

#[derive(Debug, Serialize)]
pub struct StravaStat {
    #[serde(flatten)]
    stat: Stat,
    events: i64,
}

pub async fn get_all_stats(store: &mut impl StravaStore) -> TbResult<Vec<StravaStat>> {
    let users = store.stravausers_get_all().await?;

    let mut res = Vec::new();
    for u in users {
        let stat = u.tendabike_id.get_stat(store).await?;
        let events = store.strava_events_get_count_for_user(&u.id).await?;
        res.push(StravaStat { stat, events });
    }
    Ok(res)
}

/// Returns the Strava URL for a user with the given Strava ID.
///
/// # Arguments
///
/// * `strava_id` - An `i32` representing the Strava ID of the user.
/// * `store` - A mutable reference to a `AppConn` representing the database connection.
///
/// # Returns
///
/// An `TbResult` containing a `String` representing the Strava URL for the user.
pub async fn strava_url(strava_id: i32, store: &mut impl StravaStore) -> TbResult<String> {
    let user_id = store.stravaid_get_user_id(strava_id).await?;
    Ok(format!("https://strava.com/athletes/{}", &user_id))
}
