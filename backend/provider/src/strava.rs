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

use crate::OnBehalf;
#[cfg(any(test, feature = "test-support"))]
use crate::facade::Part;
#[cfg(test)]
use crate::facade::Stat;
use crate::facade::{
    ActTypeId, Activity, ActivityId, Error, PartId, PartTypeId, Session, Summary, TbResult, UserId,
    UserStat,
};

mod traits;
pub use traits::*;

pub mod client;
pub use client::*;

pub(crate) mod session;

mod http;

pub mod activity;
pub mod event;
pub mod gear;
mod ratelimit;
// The queue's read-side rate-limit gate, re-exported so the `Store` impl
// over the data store (the sqlx crate) can consult it (spec #1029).
pub use ratelimit::limited_until;

mod user;
pub use user::*;

/// The Strava driver type (spec #1029, ticket #1032): the
/// [`Provider`](crate::driver::Provider) impl — a thin forward of the
/// driver's free functions onto the shared skeleton.
pub mod driver;
pub use driver::StravaDriver;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

fn get_time() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
