//! This module contains the implementation of the StravaUser struct and its methods.
//!
//! The StravaUser struct represents a user of the Strava API and contains information such as the user's
//! Strava ID, Tendabike ID, access token, and refresh token.
//!
//! The methods implemented for the StravaUser struct allow for reading and updating user data, as well as
//! checking the validity of the user's access token.

use diesel_derive_newtype::DieselNewType;
use newtype_derive::{newtype_fmt, NewtypeDisplay, NewtypeFrom};

use super::*;

const API: &str = "https://www.strava.com/api/v3";

#[derive(
    DieselNewType, Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct StravaId(i32);
NewtypeDisplay! { () pub struct StravaId(); }
NewtypeFrom! { () pub struct StravaId(i32); }

/// Strava User data
#[derive(Clone, Serialize, Deserialize, Queryable, Insertable, Identifiable, Debug, Default)]
#[diesel(table_name = s_diesel::schema::strava_users)]
pub struct StravaUser {
    /// the Strava user id
    pub id: StravaId,
    /// the corresponding tendabike user id
    pub tendabike_id: UserId,
    /// the time of the latest activity we have processed
    pub last_activity: i64,
    /// the access token to access user data for this user
    pub access_token: String,
    /// the expiry time for the access token
    pub expires_at: i64,
    /// the refresh token to get a new access token from Strava
    pub refresh_token: String,
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
    pub async fn read(id: UserId, conn: &mut AppConn) -> AnyResult<Self> {
        conn.read_stravauser_for_userid(id).await
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

    /// store last activity time for the user
    pub(crate) async fn update_last(&self, time: i64, conn: &mut AppConn) -> AnyResult<i64> {
        if self.last_activity >= time {
            return Ok(self.last_activity);
        }
        conn.stravauser_update_last_activity(self, time).await?;
        Ok(time)
    }

    /// check if the access token is still valid
    pub fn token_is_valid(&self) -> bool {
        self.expires_at > get_time()
    }

    fn disabled (&self) -> bool {
        self.expires_at == 0
    }

    /// update the access and optionally refresh token for the user
    ///
    /// sets a five minute buffer for the access token
    /// returns the updated user
    pub async fn update_token(
        self,
        access: &str,
        expires: Option<i64>,
        refresh: Option<&str>,
        conn: &mut AppConn,
    ) -> AnyResult<Self> {
        let iat = get_time();
        let exp = expires.unwrap() + iat - 300; // 5 Minutes buffer
        let user: StravaUser =
            conn.stravaid_update_token(self.id, access, exp, refresh).await?;

        Ok(user)
    }

    /// lock the current user
    pub async fn lock(&self, conn: &mut AppConn) -> AnyResult<()> {
        use diesel::sql_types::Bool;
        #[derive(QueryableByName, Debug)]
        struct Lock {
            #[diesel(sql_type = Bool)]
            #[diesel(column_name = pg_try_advisory_lock)]
            lock: bool,
        }

        ensure!(
            sql_query(format!("SELECT pg_try_advisory_lock({});", self.id))
                .get_result::<Lock>(conn)
                .await?
                .lock,
            Error::Conflict(format!("Two sessions for user {}", self.id))
        );
        Ok(())
    }

    /// unlock the current user
    pub async fn unlock(&self, conn: &mut AppConn) -> AnyResult<()> {
        sql_query(format!("SELECT pg_advisory_unlock({});", self.id))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub(crate) async fn request(&self, uri: &str, conn: &mut AppConn) -> AnyResult<String> {
        self.get_strava(uri, conn)
            .await?
            .text()
            .await
            .context("Could not get response body")
    }

    /// request information from the Strava API
    ///
    /// will return Error::TryAgain on certain error conditions
    /// will disable the User if Strava responds with NOT_AUTH
    async fn get_strava(&self, uri: &str, conn: &mut AppConn) -> AnyResult<reqwest::Response> {
        use reqwest::StatusCode;
        let resp = reqwest::Client::new()
            .get(format!("{}{}", API, uri))
            .bearer_auth(&self.access_token)
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
                bail!(Error::TryAgain(status.canonical_reason().unwrap()))
            }
            StatusCode::UNAUTHORIZED => {
                self.disable(conn).await?;
                bail!(Error::NotAuth(
                    "Strava request authorization withdrawn".to_string()
                ))
            }
            _ => bail!(Error::BadRequest(format!(
                "Strava request error: {}",
                status
                    .canonical_reason()
                    .unwrap_or("Unknown status received")
            ))),
        }
    }

    /// disable a user
    async fn disable(&self, conn: &mut AppConn) -> AnyResult<()> {
        let id = self.strava_id();
        info!("disabling user {}", id);
        event::insert_sync(id, crate::get_time(), conn)
            .await
            .context(format!("Could insert sync for user: {:?}", id))?;
        conn.disable_stravauser(&self.id).await
    }

    /// disable a user per admin request
    ///
    /// # Errors
    ///
    /// This function will return an error if the user does not exist, is already disabled
    /// or has open events and if strava or the database is not reachable.
    pub async fn admin_disable(self, conn: &mut AppConn) -> AnyResult<()> {
        let events = conn.get_count_of_events_for_user(&self).await?;

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

        warn!("User {} disabled by admin", self.tb_id());
        self.disable(conn).await
    }

    /// get all parts, attachments and activities for the user
    pub async fn get_summary(&self, conn: &mut AppConn) -> AnyResult<Summary> {
        use crate::*;
        gear::update_user(self, conn).await?;
        let parts = Part::get_all(self, conn).await?;
        let attachments = Attachment::for_parts(&parts, conn).await?;
        let activities = Activity::get_all(self, conn).await?;
        Ok(Summary::new(activities, parts, attachments))
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
        conn: &mut AppConn,
    ) -> AnyResult<StravaUser> {
        debug!("got id {}: {} {}", id, &firstname, &lastname);

        let user = conn.read_stravauser_for_stravaid(id)
            .await?
            .pop();
        if let Some(user) = user {
            user.tendabike_id.update(firstname, lastname, conn).await?;
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

        let user = conn.insert_stravauser(user).await?;
        sync_users(Some(user.tendabike_id), 0, conn).await?;
        Ok(user)
    }
}

impl Person for StravaUser {
    fn get_id(&self) -> UserId {
        self.tendabike_id
    }
    fn is_admin(&self) -> bool {
        false
    }
}

#[derive(Debug, Serialize)]
pub struct StravaStat {
    #[serde(flatten)]
    stat: Stat,
    events: i64,
    disabled: bool,
}

pub async fn get_all_stats(conn: &mut AppConn) -> AnyResult<Vec<StravaStat>> {
    let users = conn.get_all_stravausers().await?;

    let mut res = Vec::new();
    for u in users {
        let stat = u.tendabike_id.get_stat(conn).await?;
        let events = conn.get_count_of_events_for_user(&u).await?;
        res.push(StravaStat {
            stat,
            events,
            disabled: u.disabled(),
        });
    }
    Ok(res)
}

pub async fn sync_users(user_id: Option<UserId>, time: i64, conn: &mut AppConn) -> AnyResult<()> {
    info!("syncing users {:?} at {}", user_id, time);
    let users = match user_id {
        Some(id) => vec!(conn.read_stravauser_for_userid(id).await?),
        None => conn.get_all_stravausers().await?,
    };
    for user in users {
        if user.disabled() {
            continue;
        }
        event::insert_sync(user.id, time, conn).await?;
    }
    Ok(())
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
pub async fn strava_url(strava_id: i32, conn: &mut AppConn) -> AnyResult<String> {
    let user_id = conn.get_user_id_from_strava_id(strava_id).await?;
    Ok(format!("https://strava.com/athletes/{}", &user_id))
}
