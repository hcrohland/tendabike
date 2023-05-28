use diesel::{prelude::*, Identifiable, QueryableByName};
use diesel::{QueryDsl, RunQueryDsl, sql_query};
use diesel::{Queryable, Insertable};

use serde_derive::{Deserialize, Serialize};
use async_session::log::{info,trace,warn,debug};

pub mod activity;
pub mod gear;
pub mod event;

use kernel::{domain, s_diesel};
use anyhow::{Result as AnyResult, Context, ensure, bail};
use domain::*;

use s_diesel::AppConn;

use s_diesel::schema;
use s_diesel::schema::strava_users;


mod user;
pub use user::*;


fn get_time() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
