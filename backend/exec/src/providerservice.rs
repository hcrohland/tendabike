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

//! The provider-agnostic provider service (ADR-0014): the direct ops a
//! presentation layer drives the provider's user lifecycle and redirect URLs
//! through. The [`Registry`] implements it as a thin forward — each op opens
//! the transaction, hands the connection to the provider layer's dispatch
//! free function (the single `match provider`), and commits.
//!
//! This is the new provider-agnostic surface, built **beside** the existing
//! Strava-specific [`crate::api::ProviderService`]. The two coexist and the
//! old surface is untouched; the name `ProviderService` is shared, so the old
//! one is `crate::api::ProviderService` and this one is
//! `crate::providerservice::ProviderService`.
//!
//! Six in-transaction ops. The user-keyed ops (`sync`, `deauthorize`,
//! `delete_user`) dispatch on the user's real provider, read in-transaction
//! (the provider layer's dispatch reads the `users.provider` column) — not on
//! the route, so a per-provider route acting on a user of the other provider
//! follows the stated identity rule. The redirect-URL ops (`activity_url`,
//! `user_url` — total, `TbResult<String>` — and `gear_url`, the one non-total
//! op, `TbResult<Option<String>>` (icu gear has no public page)) are keyed by
//! the typed [`Provider`] enum.
//!
//! `#[async_trait]`; object-safe for `Arc<dyn ProviderService>`, which keeps
//! a presentation layer's state non-generic.
//!
//! [`Registry`]: crate::Registry

use tb_domain::{ActivityId, PartId, Session, TbResult, UserId};
use tb_provider::Provider;

/// The provider-agnostic provider service (ADR-0014): the six direct ops a
/// presentation layer drives the provider through. The user-keyed ops
/// (`sync`, `deauthorize`, `delete_user`) dispatch on the user's real
/// provider, read in-transaction; the redirect-URL ops are keyed by the typed
/// [`Provider`] enum. The [`Registry`] implements it as a thin forward (each
/// op opens the transaction, calls the provider layer's dispatch free
/// function, and commits — no provider dispatch in this crate).
///
/// [`Registry`]: crate::Registry
#[cfg_attr(feature = "test-support", mockall::automock)]
#[async_trait::async_trait]
pub trait ProviderService: Send + Sync {
    /// The per-user sync (ADR-0014): enqueue a `Sync` event on the user's
    /// provider queue. One transaction — the provider layer's dispatch reads
    /// the user's real provider in-transaction and enqueues the event (the
    /// Strava `strava_events` row, the icu onboarding cutover), then commits.
    async fn sync(
        &self,
        session: Box<dyn Session>,
        user_id: UserId,
        event_time: i64,
    ) -> TbResult<()>;

    /// The deauthorization (ADR-0014): one transaction — the provider layer's
    /// dispatch resolves the target's real provider in-transaction, then
    /// builds their session and deauthorizes (admin-only — the driver
    /// verifies the acting session is an admin).
    async fn deauthorize(&self, session: Box<dyn Session>, tbid: UserId) -> TbResult<()>;

    /// The user delete (ADR-0014): one transaction — the provider layer's
    /// dispatch resolves the target's real provider in-transaction, then
    /// builds their session and deletes (the deauthorization plus the row
    /// deletions).
    async fn delete_user(&self, session: Box<dyn Session>, tbid: UserId) -> TbResult<()>;

    /// The activity redirect URL (ADR-0014): one transaction — the provider
    /// layer's `activity_url` op for `id` (gated on the acting session's
    /// ownership), then the provider URL.
    async fn activity_url(
        &self,
        provider: Provider,
        session: Box<dyn Session>,
        id: ActivityId,
    ) -> TbResult<String>;

    /// The gear redirect URL (ADR-0014): one transaction — the provider
    /// layer's `gear_url` op for `id` (gated on the acting session's
    /// ownership). The one non-total op: icu gear has no public page, so the
    /// icu arm is the typed absence (`None`) — the route maps it to a no-op.
    async fn gear_url(
        &self,
        provider: Provider,
        session: Box<dyn Session>,
        id: PartId,
    ) -> TbResult<Option<String>>;

    /// The user redirect URL (ADR-0014): one transaction — the provider
    /// layer's `user_url` op for the target's tbid (no session: the route has
    /// no user context to gate on). The icu arm builds the public athlete
    /// profile page.
    async fn user_url(&self, provider: Provider, id: UserId) -> TbResult<String>;
}
