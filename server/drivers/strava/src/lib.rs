use diesel::{prelude::*, Identifiable, QueryableByName};
use diesel::{QueryDsl, RunQueryDsl, sql_query};
use diesel::{Queryable, Insertable};

use serde_derive::{Deserialize, Serialize};
use log::{info,trace,warn,debug};

pub mod activity;
pub mod gear;
pub mod event;

use kernel::{domain, s_diesel};
use anyhow::{Result, Context, Ok, ensure, bail};
use domain::*;

use s_diesel::AppConn;

use s_diesel::schema;
use s_diesel::schema::strava_users;


mod user;
pub use user::*;

mod athlete;
pub use athlete::*;

pub trait StravaContext {
    fn split(&self) -> (&StravaUser, &AppConn);
    fn conn(&self) -> &AppConn;
    fn user(&self) -> &StravaUser;
    fn request(&self, uri: &str) -> Result<String>;
}

pub fn strava_url(who: i32, context: & dyn StravaContext) -> Result<String> {
    use schema::strava_users::dsl::*;

    let user_id: i32 = strava_users
        .filter(tendabike_id.eq(who))
        .select(id)
        .first(context.conn())?;

    Ok(format!("https://strava.com/athletes/{}", &user_id))
}

fn get_time() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH).expect("Systemtime before EPOCH!")
        .as_secs().try_into().expect("Sytemtime too far in the future")
}
