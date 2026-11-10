//! This module contains the implementation of the intervals.icu driver for
//! the Tendabike server.
//!
//! The driver provides functionality to interact with the intervals.icu API,
//! including OAuth authentication and (in later tickets) fetching user
//! activities and gear. It is a feature-gated peer to the `strava` module
//! (spec #763): same feature-gate pattern, same store-trait pattern, depends
//! only on `tb_domain` via the `DomainFacade` (ADR-0013).
//!
//! The module is divided into submodules, each responsible for a specific
//! functionality:
//!
//! - `client`: the icu oauth client and its wire types (the token response
//!   and the athlete info the login exchange parses).
//! - `traits`: the `IcuDataStore` trait (the `icu_users` and `icu_events`
//!   tables) and the `IcuStore` umbrella.
//! - `user`: the `IcuUser` and the login `upsert` op.
//! - `event`: the `IcuEvent` queue row.
//!

use log::{debug, info};
use serde_derive::Serialize;

use tb_domain::*;

mod traits;
pub use traits::*;

pub mod client;
pub use client::*;

pub mod event;
pub use event::*;

pub mod user;
pub use user::*;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
