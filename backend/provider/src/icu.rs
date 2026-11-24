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
//!
//! API reference: [`docs/research/intervals-icu-api.md`](../../../docs/research/intervals-icu-api.md)
//! (required params, sort order, date format, rate-limit headers).
//!

use crate::OnBehalf;
use crate::facade::{Error, PartId, Session, TbResult, UserId, UserStat};

mod traits;
pub use traits::*;

pub mod client;
pub use client::*;

pub mod event;
pub use event::*;

pub mod user;
pub use user::*;

pub mod activity;
pub mod gear;
pub mod http;
mod ratelimit;
// The queue's read-side rate-limit gate, re-exported so the `EventQueue` impl
// over the data store (the sqlx crate) can consult it (spec #1023).
pub use ratelimit::limited_until;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
