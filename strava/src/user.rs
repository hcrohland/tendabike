//! This module contains the implementation of the StravaUser struct and its methods.
//!
//! The StravaUser struct represents a user of the Strava API and contains information such as the user's
//! Strava ID, Tendabike ID, access token, and refresh token.
//!
//! The methods implemented for the StravaUser struct allow for reading and updating user data, as well as
//! checking the validity of the user's access token.

use diesel_derive_newtype::DieselNewType;
use newtype_derive::{newtype_fmt, NewtypeDisplay, NewtypeFrom};
use serde::Deserialize;

use super::*;


#[derive(
    DieselNewType, Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct StravaId(i32);
NewtypeDisplay! { () pub struct StravaId(); }
NewtypeFrom! { () pub struct StravaId(i32); }

impl StravaId {
    /// store last activity time for the user
    pub(crate) async fn update_last(
        &self,
        time: i64,
        conn: &mut impl StravaStore,
    ) -> AnyResult<i64> {
        // if self.last_activity >= time {
        //     return Ok(self.last_activity);
        // }
        conn.stravauser_update_last_activity(self, time).await?;
        Ok(time)
    }

    /// lock the current user
    pub async fn lock(&self, conn: &mut impl StravaStore) -> AnyResult<()> {
        let lock = conn.stravaid_lock(&self).await?;
        ensure!(
            lock,
            Error::Conflict(format!("Two sessions for user {}", self))
        );
        Ok(())
    }

    /// unlock the current user
    pub async fn unlock(&self, conn: &mut impl StravaStore) -> AnyResult<usize> {
        conn.stravaid_unlock(self).await
    }

    /// update the refresh token for the user
    ///
    /// sets a five minute buffer for the access token
    /// returns the updated user
    pub async fn update_token(
        self,
        refresh: Option<&String>,
        conn: &mut impl StravaStore,
    ) -> AnyResult<StravaUser> {
        conn.stravaid_update_token(self, refresh).await
    }
}

/// Strava User data
#[derive(Clone, Serialize, Deserialize, Queryable, Insertable, Identifiable, Debug, Default)]
#[diesel(table_name = crate::schema::strava_users)]
pub struct StravaUser {
    /// the Strava user id
    id: StravaId,
    /// the corresponding tendabike user id
    tendabike_id: UserId,
    /// the time of the latest activity we have processed
    last_activity: i64,
    /// the access token to access user data for this user
    access_token: String,
    /// the expiry time for the access token
    expires_at: i64,
    /// the refresh token to get a new access token from Strava
    refresh_token: Option<String>,
}

impl StravaUser {
    /// Reads the StravaUser data for the given `id` from the database.
    ///
    /// # Arguments
    ///
    /// * `id` - A `UserId` representing the Tendabike user ID.
    /// * `conn` - A mutable reference to the database connection.
    ///
    /// # Errors
    ///
    /// Returns an `Error` if the user is not registered.
    pub async fn read(id: UserId, conn: &mut impl StravaStore) -> AnyResult<Self> {
        conn.stravauser_get_by_tbid(id).await
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

    pub fn refresh_token(&self) -> Option<String> {
        self.refresh_token.clone()
    }

    pub(crate) fn disabled(&self) -> bool {
        self.refresh_token.is_none()
    }

    /// disable a user
    async fn disable(&self, conn: &mut impl StravaStore) -> AnyResult<()> {
        let id = self.strava_id();
        info!("disabling user {}", id);
        event::insert_sync(id, self.last_activity, conn)
            .await
            .context(format!("Could insert sync for user: {:?}", id))?;
        conn.stravaid_update_token(self.id, None).await?;
        Ok(())
    }

    /// disable a user per admin request
    ///
    /// # Errors
    ///
    /// This function will return an error if the user does not exist, is already disabled
    /// or has open events and if strava or the database is not reachable.
    pub async fn admin_disable(self, conn: &mut impl StravaStore) -> AnyResult<()> {
        let events = conn.strava_events_get_count_for_user(&self.id).await?;

        if self.disabled() {
            bail!(Error::BadRequest(String::from("user already disabled!")))
        }
        if events > 0 {
            bail!(Error::BadRequest(String::from("user has open events!")))
        }

        reqwest::Client::new()
            .post("https://www.strava.com/oauth/deauthorize")
            .query(&[("access_token", &self.access_token)])
            .bearer_auth(&self.access_token)
            .send()
            .await
            .context("Could not reach strava")?
            .error_for_status()?;

        warn!("User {} disabled by admin", self.tendabike_id);
        self.disable(conn).await
    }

    /// Upsert a Strava user by ID, updating their Tendabike user ID if they already exist, or creating a new user if they don't.
    ///
    /// # Arguments
    ///
    /// * `id` - A `StravaId` representing the ID of the Strava user to upsert.
    /// * `firstname` - A `&str` representing the first name of the Strava user.
    /// * `lastname` - A `&str` representing the last name of the Strava user.
    /// * `conn` - A mutable reference to a `AppConn` representing the database connection.
    ///
    /// # Returns
    ///
    /// An `AnyResult` containing a `StravaUser` representing the upserted user.
    pub async fn upsert(
        id: StravaId,
        firstname: &str,
        lastname: &str,
        refresh: Option<&String>,
        conn: &mut impl StravaStore,
    ) -> AnyResult<StravaUser> {
        debug!("got id {}: {} {}", id, &firstname, &lastname);

        let user = conn.stravauser_get_by_stravaid(id).await?.pop();
        if let Some(user) = user {
            user.tendabike_id.update(firstname, lastname, conn).await?;
            conn.stravaid_update_token(user.id, refresh).await?;
            return Ok(user);
        }

        // create new user!
        let tendabike_id = crate::UserId::create(firstname, lastname, conn).await?;

        let user = StravaUser {
            id,
            tendabike_id,
            ..Default::default()
        };
        info!("creating new user id {:?}", user);

        let user = conn.stravauser_new(user).await?;
        event::insert_sync(user.id, 0, conn).await?;
        Ok(user)
    }

    /// Get list of gear for user from Strava
    pub async fn update_user(
        user: &mut impl StravaPerson,
        conn: &mut impl StravaStore,
    ) -> AnyResult<Vec<PartId>> {
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

        let ath: Athlete = user.request_json("/athlete", conn).await?;

        let mut parts = Vec::new();
        for gear in ath.bikes.into_iter().chain(ath.shoes) {
            parts.push(gear::strava_to_tb(gear.id, user, conn).await?);
        }

        Ok(parts)
    }
}


#[derive(Debug, Serialize)]
pub struct StravaStat {
    #[serde(flatten)]
    stat: Stat,
    events: i64,
    disabled: bool,
}

pub async fn get_all_stats(conn: &mut impl StravaStore) -> AnyResult<Vec<StravaStat>> {
    let users = conn.stravausers_get_all().await?;

    let mut res = Vec::new();
    for u in users {
        let stat = u.tendabike_id.get_stat(conn).await?;
        let events = conn.strava_events_get_count_for_user(&u.id).await?;
        res.push(StravaStat {
            stat,
            events,
            disabled: u.disabled(),
        });
    }
    Ok(res)
}

/// Returns the Strava URL for a user with the given Strava ID.
///
/// # Arguments
///
/// * `strava_id` - An `i32` representing the Strava ID of the user.
/// * `conn` - A mutable reference to a `AppConn` representing the database connection.
///
/// # Returns
///
/// An `AnyResult` containing a `String` representing the Strava URL for the user.
pub async fn strava_url(strava_id: i32, conn: &mut impl StravaStore) -> AnyResult<String> {
    let user_id = conn.stravaid_get_user_id(strava_id).await?;
    Ok(format!("https://strava.com/athletes/{}", &user_id))
}
