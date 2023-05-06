pub(crate) use anyhow::Context;

pub mod activity;
pub mod gear;

pub use crate::*;

#[derive(Error, Debug)]
pub enum OAuthError {
    #[error("authorization needed: {0}")]
    Authorize(&'static str),
}

use serde_json::Value as jValue;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct JSummary {
    activities: Vec<jValue>,
    parts: Vec<jValue>,
    attachments: Vec<jValue>
}


use diesel::{self, QueryDsl, RunQueryDsl};

use crate::{AppConn, TbResult};
use schema::strava_users;
use user::Person;
use presentation::strava::{StravaContext, webhook};

#[derive(Queryable, Insertable, Identifiable, Debug, Default)]
#[table_name = "strava_users"]
pub struct StravaUser {
    pub id: i32,
    pub tendabike_id: i32,
    pub last_activity: i64,
    pub access_token: String,
    pub expires_at: i64,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct StravaAthlete {
    firstname: String,
    lastname: String,
    id: i32
}

impl StravaAthlete {
    /// Get the user data from the Strava OAuth callback
    pub fn retrieve(self, conn: &AppConn) -> TbResult<StravaUser> {
        info!("got {:?}", self);

        let user = strava_users::table.find(self.id).get_result::<StravaUser>(conn).optional()?;
        if let Some(user) = user {
            return Ok(user);
        }

        // create user!
        let tendabike_id = crate::user::create(self.firstname, self.lastname, conn)?;

        let user = StravaUser {
            id: self.id,
            tendabike_id,
            ..Default::default()
        };

        let user = diesel::insert_into(strava_users::table)
            .values(&user)
            .get_result(conn)?;
        webhook::insert_sync(self.id, 0, conn)?;
        Ok(user)
    }

}

impl StravaUser {
    pub fn is_valid (&self) -> bool {
        self.expires_at > time::get_time().sec 
    }
    
    pub fn read (id: i32, conn: &AppConn) -> TbResult<Self> {
        Ok(strava_users::table
            .filter(strava_users::tendabike_id.eq(id))
            .get_result(conn)
            .context(format!("User::get: user {} not registered", id))?)
    }

    pub fn update_db(&self, conn: &AppConn) -> TbResult<()> {
        use schema::strava_users::dsl::*;
        diesel::update(strava_users.find(self.id))
            .set((expires_at.eq(0), access_token.eq("")))
            .execute(conn).context(format!("Could not disable record for user {}",self.id))?;
        Ok(())
    }

    pub fn tb_id(&self) -> i32 {
        self.tendabike_id
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn last_activity(&self) -> i64 {
        self.last_activity
    }

    pub fn update_last(&self, time: i64, conn: &AppConn) -> TbResult<i64> {
        if self.last_activity >= time {
            return Ok(self.last_activity);
        }
        use schema::strava_users::dsl::*;

        diesel::update(strava_users.find(self.id))
            .set(last_activity.eq(time))
            .execute(conn).context("Could not update last_activity")?;
        Ok(time)
    }

    pub fn update(self, access: &str, expires: Option<i64>, refresh: Option<&str>, conn: &AppConn) -> TbResult<Self> {
        use schema::strava_users::dsl::*;
        use time::*;
        
        let iat = get_time().sec;
        let exp = expires.unwrap() as i64 + iat - 300; // 5 Minutes buffer
        let user: StravaUser = diesel::update(strava_users.find(self.id()))
            .set((
                access_token.eq(access),
                expires_at.eq(exp),
                refresh_token.eq(refresh.unwrap()),
            ))
            .get_result(conn).context("Could not store user")?;
        
        Ok(user)
    }
}

impl Person for StravaUser {
    fn get_id(&self) -> i32 {
        self.tendabike_id
    }
    fn is_admin(&self) -> bool {
        false
    }
}


pub fn strava_url(who: i32, context: &StravaContext) -> TbResult<String> {
    use schema::strava_users::dsl::*;

    let g: i32 = strava_users
        .filter(tendabike_id.eq(who))
        .select(id)
        .first(context.conn())?;

    Ok(format!("https://strava.com/athletes/{}", &g))
}

pub fn get_stats(tbid: i32, conn: &AppConn) -> TbResult<(i64, bool)> {
    use schema::strava_events::dsl::*;

    let user: StravaUser = strava_users::table
        .filter(strava_users::tendabike_id.eq(tbid))
        .get_result(conn)
        .context(format!("get_stats: tb user {} not registered", tbid))?;

    let events = strava_events.count().filter(owner_id.eq(user.tendabike_id)).first(conn)?;
    return Ok((events, user.expires_at == 0))
}

/// Get the strava id for all users
pub fn getusers (user_id: Option<i32>, conn: &AppConn) -> TbResult<Vec<i32>> {
    use schema::strava_users::dsl::*;

    Ok(
        match user_id {
            Some(user ) => strava_users.filter(tendabike_id.eq(user)).select(id).get_results(conn)?,
            None => strava_users.select(id).get_results(conn)?
        }
    )
}