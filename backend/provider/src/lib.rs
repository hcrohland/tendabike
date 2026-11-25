/*
   tendabike - the bike maintenance tracker

   Copyright (C) 2023  Christoph Rohland

   This program is free software: you can redistribute it and/or modify
   it under the terms of the GNU Affero General Public License as published
   by the Free Software Foundation, either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU Affero General Public License for more details.

   You should have received a copy of the GNU Affero General Public License
   along with this program.  If not, see <https://www.gnu.org/licenses/>.

*/

#![forbid(unsafe_code)]
#![warn(unreachable_pub)]

//! The provider-agnostic seams shared by every activity provider (spec #781).
//!
//! [`activity_id`] — [`ProviderActivityId`], the cross-provider typed activity
//! id: the form the domain's opaque `ActivityId` converts to and from, and the
//! single home for the `strava:` / `icu:` prefix knowledge (ADR-0013).
//!
//! [`facade`] — [`EntityFacade`], the trait a provider goes through to reach
//! the domain, and its concrete implementation. A provider function is generic
//! over `D: EntityFacade` (static dispatch); tests swap in the
//! [`MockEntityFacade`] (under `test-support`).
//!
//! [`lane`] — [`ProviderLane`] + its [`Probe`] outcome, the narrow seam the
//! executor loop calls through to probe and process a provider's event queue.
//! The unified lane is the seam's single impl (ADR-0014, amended); the
//! executor is generic over the lane it is given and never names a concrete
//! provider.
//!
//! [`queue`] — the provider-agnostic event-queue kernel: the queue's
//! read-side rules (the rate-limit gate, the latest-per-key dedup) and the
//! four-arm failure policy, written once and generic over the
//! [`ProviderSpec`](crate::ProviderSpec) marker, driven through the
//! [`Store`](crate::store::Store) interface (spec #1029).
//!
//! [`driver`] — [`Provider`](crate::driver::Provider), the provider driver:
//! the single driver interface, generic over the marker — the
//! provider-specific core the author writes plus the shared user-lifecycle
//! skeleton as default methods (spec #1029). Namespaced under `driver` (not
//! re-exported at the crate root) beside the [`Provider`] enum.
//!
//! [`spec`] — [`ProviderSpec`], the provider marker: the type-only carrier of
//! a provider's wire shapes (the user row, the event row, the owner id, the
//! event key, the sync-event constructor, and the identity accessors).
//!
//! [`store`] — [`Store`], the provider-agnostic store: the single uniform
//! store interface, generic over the marker, carrying the user-row and the
//! event-row operations (spec #1029).
//!
//! [`unified`] — the provider lane: the [`ProviderLane`] impl the executor
//! loop runs in production. Generic over the provider marker and driver, it
//! is bounded on the single [`Store`](crate::store::Store) interface and
//! drives the [`Provider`](crate::driver::Provider) trait methods, so it is
//! provider-agnostic. Provider selection happens at the call site (the
//! dispatch), not in the lane.

mod config;
pub use config::{ProviderConfig, disabled, enabled_providers, init};

mod activity_id;
pub use activity_id::*;

mod provider;
pub use provider::*;

mod facade;
pub use facade::*;

mod lane;
pub use lane::*;

#[cfg(all(feature = "strava", feature = "icu"))]
mod unified;
#[cfg(all(feature = "strava", feature = "icu"))]
pub use unified::*;

/// The provider-agnostic direct-op dispatch (ADR-0014): the eight free
/// functions the executor's provider-agnostic `ProviderService` impl forwards
/// to — a sibling of the unified lane seam. A public module (not re-exported
/// at the crate root) so the dispatch's generic names (`sync`, `delete_user`,
/// …) stay namespaced under `dispatch` rather than polluting the crate root.
#[cfg(all(feature = "strava", feature = "icu"))]
pub mod dispatch;

mod ratelimit;
pub(crate) use ratelimit::{Cadence, RateLimitShim, RateLimiter, UsageKind, WindowSpec};

mod failure;
pub use failure::*;

mod queue;

mod spec;
pub use spec::ProviderSpec;

mod store;
pub use store::Store;

/// The provider driver trait (spec #1029, ticket #1032): the single driver
/// interface, generic over the marker — the provider-specific core the author
/// writes plus the shared user-lifecycle skeleton as default methods. A
/// public module (not re-exported at the crate root) so the trait's name
/// (`Provider`) stays namespaced under `driver` rather than colliding with
/// the crate root's [`Provider`] enum.
pub mod driver;

mod http;
pub use http::*;

mod session;
pub(crate) use session::*;

mod client;
pub use client::*;

mod activity_type;
pub use activity_type::*;

#[cfg(feature = "strava")]
pub mod strava;

#[cfg(feature = "icu")]
pub mod icu;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
