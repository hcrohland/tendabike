use diesel::{prelude::*, Identifiable, QueryableByName};
use diesel::{QueryDsl, RunQueryDsl, sql_query};
use diesel::{Queryable, Insertable};

use kernel::stravatrait::StravaStore;
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

pub fn strava_url(strava_id: i32, store: &dyn StravaStore) -> Result<String> {
    let user_id = store.get_user_id_from_strava_id(strava_id)?;
    Ok(format!("https://strava.com/athletes/{}", &user_id))
}

fn get_time() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH).expect("Systemtime before EPOCH!")
        .as_secs().try_into().expect("Sytemtime too far in the future")
}
