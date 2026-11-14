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
//! [`unified`] — the unified lane (ADR-0014): the [`ProviderLane`] impl,
//! keyed by the [`LaneEvent`] the two providers' event rows share. The lane
//! the executor loop runs in production: it selects between the two
//! providers per call, with a typed switch on [`Provider`], and calls the
//! driver functions directly.

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

mod http;
pub use http::*;

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
