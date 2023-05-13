use diesel::{prelude::*, Identifiable, QueryableByName};
use diesel::{QueryDsl, RunQueryDsl, sql_query};
use diesel::{Queryable, Insertable};

use serde_derive::{Deserialize, Serialize};
use serde_json::Value as jValue;
use log::{info,trace,warn,debug};

pub mod activity;
pub mod gear;
pub mod event;

use kernel::domain::*;
use kernel::s_diesel;
use kernel::error::Error;
use anyhow::{Result, Context, Ok, ensure, bail};

use kernel::domain::presentation::Person;
use s_diesel::AppConn;

use s_diesel::schema;
use s_diesel::schema::strava_users;



pub trait StravaContext {
    fn split(&self) -> (&StravaUser, &AppConn);
    fn conn(&self) -> &AppConn;
    fn user(&self) -> &StravaUser;
    fn request(&self, uri: &str) -> Result<String>;
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct JSummary {
    activities: Vec<jValue>,
    parts: Vec<jValue>,
    attachments: Vec<jValue>
}

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
    pub fn retrieve(self, conn: &AppConn) -> Result<StravaUser> {
        info!("got {:?}", self);

        let user = strava_users::table.find(self.id).get_result::<StravaUser>(conn).optional()?;
        if let Some(user) = user {
            return Ok(user);
        }

        // create user!
        let tendabike_id = crate::User::create(self.firstname, self.lastname, conn)?;

        let user = StravaUser {
            id: self.id,
            tendabike_id,
            ..Default::default()
        };

        let user: StravaUser = diesel::insert_into(strava_users::table)
            .values(&user)
            .get_result(conn)?;
        sync_users(Some(user.id), 0, conn)?;
        Ok(user)
    }
}

impl StravaUser {
    pub fn is_valid (&self) -> bool {
        self.expires_at > get_time()
    }
    
    pub fn read (id: i32, conn: &AppConn) -> Result<Self> {
        Ok(strava_users::table
            .filter(strava_users::tendabike_id.eq(id))
            .get_result(conn)
            .context(format!("User::get: user {} not registered", id))?)
    }

    pub fn update_db(&self, conn: &AppConn) -> Result<()> {
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

    fn update_last(&self, time: i64, conn: &AppConn) -> Result<i64> {
        if self.last_activity >= time {
            return Ok(self.last_activity);
        }
        use schema::strava_users::dsl::*;

        diesel::update(strava_users.find(self.id))
            .set(last_activity.eq(time))
            .execute(conn).context("Could not update last_activity")?;
        Ok(time)
    }

    pub fn update(self, access: &str, expires: Option<i64>, refresh: Option<&str>, conn: &AppConn) -> Result<Self> {
        use schema::strava_users::dsl::*;
        
        let iat = get_time();
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

    pub fn lock (&self, conn: &AppConn) -> Result<()> {
        use diesel::sql_types::Bool;
        #[derive(QueryableByName, Debug)]
        struct Lock {
            #[sql_type = "Bool"]
            #[column_name = "pg_try_advisory_lock"]
            lock: bool
        }
    
        ensure!(
            sql_query(format!("SELECT pg_try_advisory_lock({});", self.id)).get_result::<Lock>(conn)?.lock,
            Error::Conflict(format!("Two sessions for user {}", self.id))
        );
        Ok(())
    }
    
    pub fn unlock(&self, conn: &AppConn) -> Result<()> {
        sql_query(format!("SELECT pg_advisory_unlock({});", self.id)).execute(conn)?;
        Ok(())
    }        

    /// return the open events and the disabled status for a user.
    ///
    /// # Errors
    ///
    /// This function will return an error if the database connection fails.
    pub fn get_stats(&self, conn: &AppConn) -> Result<(i64, bool)> {
        use schema::strava_events::dsl::*;

        let events = strava_events.count().filter(owner_id.eq(self.tendabike_id)).first(conn)?;
        return Ok((events, self.expires_at == 0))
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

pub fn strava_url(who: i32, context: & dyn StravaContext) -> Result<String> {
    use schema::strava_users::dsl::*;

    let user_id: i32 = strava_users
        .filter(tendabike_id.eq(who))
        .select(id)
        .first(context.conn())?;

    Ok(format!("https://strava.com/athletes/{}", &user_id))
}

#[derive(Debug, Serialize)]
pub struct StravaStat {
    #[serde(flatten)]
    stat: Stat,
    events: i64,
    disabled: bool,
}

pub fn get_all_stats(conn: &AppConn) -> Result<Vec<StravaStat>> {
    let users = strava_users::table
        .get_results::<StravaUser>(conn)
        .context(format!("get_stats: could not read users"))?;

    users.into_iter().map(|u| {
        let uid = u.tendabike_id;
        let stat = User::get_stat(uid, conn)?;
        let (events, disabled) = u.get_stats(conn)?;
        Ok(StravaStat {stat, events, disabled})
    }).collect()
}

/// Get the strava id for all users
pub fn sync_users (user_id: Option<i32>, time: i64, conn: &AppConn) -> Result<()> {
    use schema::strava_users::dsl::*;

    let users =
        match user_id {
            Some(user ) => strava_users.filter(tendabike_id.eq(user)).select(id).get_results(conn)?,
            None => strava_users.select(id).get_results(conn)?
        };
        for user_id in users {
            event::insert_sync(user_id, time, &conn)?;
        };
        Ok(())
}

pub fn user_summary(context: & dyn StravaContext) -> Result<Summary> {
    use crate::*;
    gear::update_user(context)?;
    let (user, conn) = context.split();
    let parts = Part::get_all(user, conn)?;
    let attachments = Attachment::for_parts(&parts,&conn)?;
    let activities = Activity::get_all(user, conn)?;
    Ok(Summary::new(activities, parts,attachments))
}

fn get_time() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH).expect("Systemtime before EPOCH!")
        .as_secs().try_into().expect("Sytemtime too far in the future")
}