//! This module contains the implementation of the Strava driver for Tendabike server.
//!
//! The driver provides functionality to interact with Strava API, including authentication, fetching user data, and fetching user activities.
//!
//! The module is divided into submodules, each responsible for a specific functionality:
//!
//! - `activity`: contains functionality to fetch user activities from Strava API.
//! - `event`: contains functionality to handle Strava webhook events.
//! - `gear`: contains functionality to fetch user gear data from Strava API.
//! - `user`: contains functionality to fetch user data from Strava API and manage user authentication.
//!
//! The module also contains some utility functions and imports used across the submodules.
//!
//! # Examples
//!

use diesel::{prelude::*, Identifiable, QueryableByName};
use diesel::{QueryDsl, sql_query};
use diesel::{Queryable, Insertable};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};

use serde_derive::{Deserialize, Serialize};
use async_session::log::{info,trace,warn,debug};


pub mod activity;
pub mod gear;
pub mod event;

use anyhow::{Result as AnyResult, Context, ensure, bail};
use domain::*;

pub mod strava_store;

use s_diesel::AppConn;

use s_diesel::schema;


mod user;
pub use user::*;


fn get_time() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
