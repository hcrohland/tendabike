//! This module contains the implementation of the intervals.icu driver for
//! the Tendabike server.
//!
//! The driver provides functionality to interact with the intervals.icu API,
//! including OAuth authentication, webhook acceptance, and activity import
//! (spec #763/#764). It is a feature-gated peer to the `strava` module: same
//! feature-gate pattern, same store-trait pattern, depends only on
//! `tb_domain` via the `EntityFacade` (ADR-0013).
//!
//! The module is divided into submodules, each responsible for a specific
//! functionality:
//!
//! - `client`: the icu oauth client and its wire types (the token response
//!   and the athlete info the login exchange parses).
//! - `traits`: the `IcuDataStore` trait (the `icu_users` and `icu_events`
//!   tables) and the `IcuStore` umbrella.
//! - `user`: the `IcuUser` and the login `upsert` op.
//! - `event`: the `IcuEvent` queue row and the queue's read/process ops.
//! - `http`: the thin icu HTTP calls (the bearer GET, the 401/429 mapping).
//! - `activity`: the icu activity wire shape, its mapping to the domain, the
//!   type mapping, and stub detection.
//! - `gear`: the icu gear wire shape, its type mapping, and the import of the
//!   user's gear list (`GET /athlete/{id}/gear`).
//! - `ratelimit`: the icu rate-limit shim over the shared kernel.
//! - `lane`: the `IcuLane` (`ProviderLane` impl).
//!

use log::{debug, info};
use serde_derive::{Deserialize, Serialize};

use tb_domain::*;

mod traits;
pub use traits::*;

pub mod client;
pub use client::*;

pub mod event;
pub use event::*;

pub mod user;
pub use user::*;

pub mod activity;
mod backfill;
pub mod gear;
pub mod http;
mod ratelimit;

mod lane;
pub use lane::*;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

/// A minimal, identity-only [`Session`] for the event lane (ADR-0011): the
/// event lane has no acting session, so where it crosses into a session-typed
/// domain call it constructs this adapter. Personal, non-admin, no shop —
/// the event lane's actual context (ADR-0009).
#[derive(Debug, Clone, Copy)]
struct OnBehalf(UserId);

impl Session for OnBehalf {
    fn user_id(&self) -> UserId {
        self.0
    }
    fn shop(&self) -> Option<ShopId> {
        None
    }
    fn is_admin(&self) -> bool {
        false
    }
}
