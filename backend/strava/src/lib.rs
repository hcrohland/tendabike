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

use anyhow::Context;
use async_session::log::{debug, info, trace, warn};
use serde_derive::{Deserialize, Serialize};

use tb_domain::*;

mod traits;
pub use traits::*;

pub mod activity;
pub mod event;
pub mod gear;

mod user;
pub use user::*;

fn get_time() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp()
}
