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
//! The in-transaction ops. The target-keyed ops (`sync`, `deauthorize`,
//! `delete_user`, `gear_url`, `user_url`) dispatch on the target's real
//! provider, derived in-transaction from the provider-internal data (the
//! provider layer's `provider_of` lookup — `gear_url` from the part's owner,
//! the rest from the user's tbid — the domain stores no provider) — not on
//! the route, so a per-provider route acting on a target of the other
//! provider follows the stated identity rule. The route-keyed ops (`login`,
//! `accept_event`) take the typed [`Provider`] as a parameter (the route
//! knows it from the URL) plus opaque string input (the authorization code,
//! scope, and PKCE code verifier, the raw webhook body) — no driver wire
//! type crosses the seam.
//! The redirect-URL ops: `activity_url` is keyed by the provider parsed from
//! the activity id's prefix; `user_url` (total, `TbResult<String>`) and
//! `gear_url` (the one non-total op, `TbResult<Option<String>>` — icu gear
//! has no public page) derive the target's real provider in the op's own
//! transaction.
//!
//! `#[async_trait]`; object-safe for `Arc<dyn ProviderService>`, which keeps
//! a presentation layer's state non-generic.
//!
//! [`Registry`]: crate::Registry

use tb_provider::Provider;

use crate::facade::{ActivityId, PartId, Session, TbResult, UserId};

/// The provider-agnostic provider service (ADR-0014): the direct ops a
/// presentation layer drives the provider through. The target-keyed ops
/// (`sync`, `deauthorize`, `delete_user`, `gear_url`, `user_url`) dispatch on
/// the target's real provider, derived in-transaction from the
/// provider-internal data; the route-keyed ops (`login`, `accept_event`) take
/// the typed [`Provider`] as a parameter, and `activity_url` is keyed by the
/// provider parsed from the activity id's prefix. The [`Registry`] implements
/// it as a thin forward (each op opens the transaction, calls the provider
/// layer's dispatch free function, and commits — no provider dispatch in this
/// crate).
///
/// [`Registry`]: crate::Registry
#[cfg_attr(feature = "test-support", mockall::automock)]
#[async_trait::async_trait]
pub trait ProviderService: Send + Sync {
    /// The login (ADR-0014): one transaction — the provider layer's dispatch
    /// does the provider's login work (the scope check, the token exchange,
    /// the athlete wire-shape parsing, and the upsert), returning the
    /// identity and the admin flag. The provider is the route's (it knows it
    /// from the URL); `code`, `scope`, and `verifier` are the opaque callback
    /// values — no driver wire type crosses the seam. `verifier` is the PKCE
    /// code verifier (intervals.icu-only — the Strava flow ignores it).
    async fn login<'a>(
        &self,
        provider: Provider,
        code: &'a str,
        scope: &'a str,
        verifier: Option<&'a str>,
    ) -> TbResult<(UserId, bool)>;

    /// The webhook intake (ADR-0014): one transaction — the provider
    /// layer's dispatch does the provider's intake work (the envelope parse,
    /// the secret verification, the event filtering, the timestamp parse, and
    /// the queue insert), then the registry wakes each queued user
    /// (fire-and-forget). `body` is the raw webhook body — no driver wire
    /// type crosses the seam.
    async fn accept_event(&self, provider: Provider, body: &str) -> TbResult<()>;

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
    /// ownership — the part read is the check). The provider is the part's
    /// own: derived in-transaction from the part's owner (the same identity
    /// rule as `deauthorize` / `delete_user`). The one non-total op: icu
    /// gear has no public page, so the icu arm is the typed absence
    /// (`None`) — the route maps it to a no-op.
    async fn gear_url(&self, session: Box<dyn Session>, id: PartId) -> TbResult<Option<String>>;

    /// The user redirect URL (ADR-0014): one transaction — the provider
    /// layer's `user_url` op for the target's tbid (no session: the route has
    /// no user context to gate on). The provider is the target's own:
    /// derived in-transaction from the target's data (the same identity rule
    /// as `deauthorize` / `delete_user`). The icu arm builds the public
    /// athlete profile page.
    async fn user_url(&self, id: UserId) -> TbResult<String>;

    /// The user's provider identity (ADR-0014): one transaction — the
    /// provider layer's `provider_of` lookup, derived from the
    /// provider-internal user data (the domain stores no provider). The web
    /// layer's `GET /user` response calls it to populate the client's
    /// `user.provider` field.
    async fn user_provider(&self, id: UserId) -> TbResult<Provider>;

    /// The enabled providers (ADR-0016): the runtime, ordered subset of
    /// [`Provider`] (the enum's definition order) the server was
    /// configured with — the provider layer's config-derived set. No
    /// transaction: a pure config read. The web layer's `GET /api/providers`
    /// answers with it, so the client offers only the configured providers.
    fn enabled_providers(&self) -> Vec<Provider>;
}
