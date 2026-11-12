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

//! The provider-agnostic direct-op dispatch (ADR-0014): the eight free
//! functions the executor's provider-agnostic [`ProviderService`] impl
//! forwards to. Each is a typed switch on [`Provider`] that delegates to the
//! driver module (or the driver's [`ProviderLane`]) that does the actual work
//! — the single home for the provider knowledge on the direct-op path, a
//! sibling of the unified lane seam (`unified`). The executor is a thin
//! forward: it opens the transaction, hands the connection to one of these,
//! and commits; no `match provider` and no driver name appear in `tb_exec`.
//!
//! The user-keyed ops (`sync`, `deauthorize`, `delete_user`) dispatch on the
//! user's real provider, read in-transaction (the [`provider_of`] read — the
//! same identity rule the unified lane uses), not on the route: a
//! per-provider route acting on a user of the other provider follows the
//! stated identity rule instead of failing on a missing lookup.
//!
//! The store is a complete store for both providers (`StravaStore +
//! IcuStore`): each arm bounds on its own driver's store, and the switch is
//! exhaustive over the enum.
//!
//! [`ProviderService`]: crate (re-exported in `tb_exec`)

use tb_domain::{ActivityId, PartId, Session, TbResult, UserId};

use crate::EntityFacadeImpl;
use crate::Provider;
use crate::ProviderLane;
use crate::icu::{IcuLane, IcuStore, event::get_icu_id};
use crate::strava::{StravaLane, StravaStore, gear, strava_url, user_deauthorize, user_delete};

/// The user's provider, read from the `users.provider` column (through the
/// owning entity's `UserId::provider` read, ADR-0010) and parsed into the
/// typed [`Provider`] (the single dispatch mechanism, ADR-0014) — the same
/// identity rule the unified lane uses. A read failure is a DB failure the
/// caller propagates.
async fn provider_of<S>(store: &mut S, user_id: UserId) -> TbResult<Provider>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    let s = user_id.provider(store).await?;
    Provider::try_from(s.as_str())
}

/// The provider-agnostic `login` dispatch (ADR-0014): the provider's login
/// work — the scope check, the token exchange, the athlete wire-shape
/// parsing, and the upsert — returning the identity and the admin flag. The
/// provider is the route's (it knows it from the URL); `code` and `scope`
/// are the opaque callback values (no driver wire type crosses the seam).
pub async fn login<S>(
    provider: Provider,
    code: &str,
    scope: &str,
    store: &mut S,
) -> TbResult<(UserId, bool)>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    match provider {
        Provider::Strava => {
            crate::strava::login(code, scope, &crate::EntityFacadeImpl, store).await
        }
        Provider::Icu => crate::icu::login(code, scope, &crate::EntityFacadeImpl, store).await,
    }
}

/// The provider-agnostic `accept_event` dispatch (ADR-0014): the provider's
/// webhook intake work — the envelope parse, the secret verification, the
/// event filtering, the timestamp parse, and the queue insert — returning
/// the TendaBike users a row was queued for (the caller's wake list). The
/// provider is the route's (it knows it from the URL); `body` is the raw
/// webhook body (no driver wire type crosses the seam).
pub async fn accept_event<S>(provider: Provider, body: &str, store: &mut S) -> TbResult<Vec<UserId>>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    match provider {
        Provider::Strava => {
            let queued = crate::strava::event::accept_event(body, store).await?;
            Ok(queued.into_iter().collect())
        }
        Provider::Icu => crate::icu::event::accept_event(body, store).await,
    }
}

/// The provider-agnostic `sync` dispatch (ADR-0014): enqueue a `Sync` event
/// on the user's provider queue. The provider is the user's real one, read
/// in-transaction (a per-provider route acting on a user of the other
/// provider follows the stated identity rule). The Strava arm queues the
/// `strava_events` row; the icu arm runs the icu onboarding cutover (gear
/// import + backfill) through its lane.
pub async fn sync<S>(user_id: UserId, event_time: i64, store: &mut S) -> TbResult<()>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    let provider = provider_of(store, user_id).await?;
    match provider {
        Provider::Strava => {
            StravaLane::new(EntityFacadeImpl)
                .insert_sync(user_id, event_time, false, store)
                .await
        }
        Provider::Icu => {
            IcuLane::new(EntityFacadeImpl)
                .insert_sync(user_id, event_time, false, store)
                .await
        }
    }
}

/// The provider-agnostic `deauthorize` dispatch (ADR-0014): revoke the
/// target's provider token and disable their local row (admin-only — the
/// driver verifies the acting session is an admin). The provider is the
/// target's real one, read in-transaction.
pub async fn deauthorize<S>(session: &mut dyn Session, tbid: UserId, store: &mut S) -> TbResult<()>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    let provider = provider_of(store, tbid).await?;
    match provider {
        Provider::Strava => user_deauthorize(session, tbid, store).await,
        Provider::Icu => todo!("icu deauthorize"),
    }
}

/// The provider-agnostic `delete_user` dispatch (ADR-0014): deauthorize the
/// target (admin-only) plus delete their provider and domain rows. The
/// provider is the target's real one, read in-transaction.
pub async fn delete_user<S>(session: &mut dyn Session, tbid: UserId, store: &mut S) -> TbResult<()>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    let provider = provider_of(store, tbid).await?;
    match provider {
        Provider::Strava => user_delete(session, tbid, &crate::EntityFacadeImpl, store).await,
        Provider::Icu => todo!("icu delete user"),
    }
}

/// The provider-agnostic `activity_url` dispatch (ADR-0014): the provider's
/// URL for `id`, gated on the acting session's ownership (the read is the
/// ownership check).
pub async fn activity_url<S>(
    provider: Provider,
    session: Box<dyn Session>,
    id: ActivityId,
    store: &mut S,
) -> TbResult<String>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    match provider {
        Provider::Strava => {
            StravaLane::new(EntityFacadeImpl)
                .activity_url(id, session, store)
                .await
        }
        Provider::Icu => {
            IcuLane::new(EntityFacadeImpl)
                .activity_url(id, session, store)
                .await
        }
    }
}

/// The provider-agnostic `gear_url` dispatch (ADR-0014): the provider's gear
/// page for the part `id`, gated on the acting session's ownership. The one
/// non-total op: icu gear is a private settings item with no public page, so
/// the icu arm is the typed absence (`None`) — the route maps it to a no-op.
pub async fn gear_url<S>(
    provider: Provider,
    session: Box<dyn Session>,
    id: PartId,
    store: &mut S,
) -> TbResult<Option<String>>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    match provider {
        Provider::Strava => {
            gear::strava_url(i32::from(id), session, &crate::EntityFacadeImpl, store).await
        }
        Provider::Icu => Ok(None),
    }
}

/// The provider-agnostic `user_url` dispatch (ADR-0014): the provider's
/// public athlete profile URL for the target `tbid` (no session: the route
/// has no user context to gate on). The icu arm builds the public athlete
/// profile page from the icu user row's athlete id.
pub async fn user_url<S>(provider: Provider, id: UserId, store: &mut S) -> TbResult<String>
where
    S: StravaStore + IcuStore + Send + Sync + 'static,
{
    match provider {
        Provider::Strava => strava_url(id, store).await,
        Provider::Icu => {
            let icu_id = get_icu_id(id, store).await?;
            Ok(format!("https://intervals.icu/athletes/{icu_id}"))
        }
    }
}

#[cfg(test)]
mod dispatch_tests;
