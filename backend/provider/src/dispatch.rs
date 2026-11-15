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
//! driver module that does the actual work — the single home for the
//! provider knowledge on the direct-op path, a sibling of the unified lane
//! seam (`unified`). The executor is a thin forward: it opens the
//! transaction, hands the connection (and the entity-op facade) to one of
//! these, and commits; no `match provider` and no driver name appear in
//! `tb_exec`.
//!
//! The user-keyed ops (`sync`, `deauthorize`, `delete_user`) dispatch on the
//! user's real provider, derived in-transaction from the provider-internal
//! user data (the [`provider_of`](crate::provider_of) lookup — the same
//! identity rule the unified lane uses), not on the route: a
//! per-provider route acting on a user of the other provider follows the
//! stated identity rule instead of failing on a missing lookup.
//!
//! The store is the local data store for both providers
//! (`StravaDataStore + IcuDataStore`): each arm bounds on its own driver's
//! data store, and the switch is exhaustive over the enum. Domain work goes
//! through the entity-op facade (a parameter — the registry passes the
//! concrete one); the pure queue-intake ops (`accept_event`, `user_url`)
//! need no facade.
//!
//! [`ProviderService`]: crate (re-exported in `tb_exec`)

use crate::facade::{ActivityId, PartId, Session, TbResult, UserId};

use crate::EntityFacade;
use crate::Provider;

/// The provider-agnostic `login` dispatch (ADR-0014): the provider's login
/// work — the scope check, the token exchange, the athlete wire-shape
/// parsing, and the upsert — returning the identity and the admin flag. The
/// provider is the route's (it knows it from the URL); `code` and `scope`
/// are the opaque callback values (no driver wire type crosses the seam).
pub async fn login<S, D: EntityFacade<S>>(
    provider: Provider,
    code: &str,
    scope: &str,
    facade: &D,
    store: &mut S,
) -> TbResult<(UserId, bool)>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    match provider {
        Provider::Strava => crate::strava::login(code, scope, facade, store).await,
        Provider::Icu => crate::icu::login(code, scope, facade, store).await,
    }
}

/// The provider-agnostic `accept_event` dispatch (ADR-0014): the provider's
/// webhook intake work — the envelope parse, the secret verification, the
/// event filtering, the timestamp parse, and the queue insert — returning
/// the TendaBike users a row was queued for (the caller's wake list). The
/// provider is the route's (it knows it from the URL); `body` is the raw
/// webhook body (no driver wire type crosses the seam). A pure queue-intake
/// op: it drives no domain work, so it takes no facade.
pub async fn accept_event<S>(provider: Provider, body: &str, store: &mut S) -> TbResult<Vec<UserId>>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    match provider {
        Provider::Strava => {
            let queued = crate::strava::event::accept_event(body, store).await?;
            Ok(queued.into_iter().collect())
        }
        Provider::Icu => crate::icu::event::accept_event(body, store).await,
    }
}

/// The sync switch (ADR-0014): the single home for the user's provider
/// identity read plus the Strava gear-import-and-`insert_sync(…, false, …)`
/// / icu `onboard` arms. The provider is the user's real one, read
/// in-transaction (a per-provider route acting on a user of the other
/// provider follows the stated identity rule). The admin trigger delegates
/// here (the registry's [`sync`](crate::dispatch::sync)): one `match` for the
/// combined gear-import-plus-queue sync op.
///
/// Returns the created gear part ids (spec #829): the Strava arm imports the
/// gear (bikes + shoes) before queuing the activity sync, and the icu arm
/// returns the gear its onboarding imported. The admin trigger drops them.
///
/// The onboarding's two steps do not run through this combined op: they use
/// the split [`import_gear_user`](crate::dispatch::import_gear_user) and
/// [`queue_sync_user`](crate::dispatch::queue_sync_user), so the gear import
/// and the activity queue are separate decisions (spec #829).
pub(crate) async fn sync_user<S, D: EntityFacade<S>>(
    user_id: UserId,
    event_time: i64,
    facade: &D,
    store: &mut S,
) -> TbResult<Vec<PartId>>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    let provider = crate::provider_of(store, user_id).await?;
    match provider {
        Provider::Strava => {
            // Import the gear (bikes + shoes) before queuing the activity
            // sync (spec #829): the gear is the onboarding's provider side.
            let gear = crate::strava::StravaUser::update_gear(user_id, facade, store).await?;
            crate::strava::event::insert_sync(user_id, event_time, false, store).await?;
            Ok(gear)
        }
        Provider::Icu => crate::icu::onboard(user_id, event_time, facade, store).await,
    }
}

/// The onboarding gear import (spec #829, the first step): import the user's
/// gear (bikes + shoes) from the provider, returning the created part ids. No
/// activity sync is queued — the user decides on that next (the second step,
/// [`queue_sync_user`](crate::dispatch::queue_sync_user)). The provider is the
/// user's real one, read in-transaction.
pub(crate) async fn import_gear_user<S, D: EntityFacade<S>>(
    user_id: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<Vec<PartId>>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    let provider = crate::provider_of(store, user_id).await?;
    match provider {
        Provider::Strava => crate::strava::StravaUser::update_gear(user_id, facade, store).await,
        Provider::Icu => crate::icu::gear::update_gear(user_id, facade, store).await,
    }
}

/// The onboarding activity sync (spec #829, the second step): queue the
/// activity sync on the user's provider queue (the Strava `strava_events` row,
/// the icu date-window backfill). No gear import — that was the first step
/// ([`import_gear_user`](crate::dispatch::import_gear_user)). The provider is
/// the user's real one, read in-transaction.
pub(crate) async fn queue_sync_user<S, D: EntityFacade<S>>(
    user_id: UserId,
    event_time: i64,
    facade: &D,
    store: &mut S,
) -> TbResult<()>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    let provider = crate::provider_of(store, user_id).await?;
    match provider {
        Provider::Strava => {
            crate::strava::event::insert_sync(user_id, event_time, false, store).await
        }
        Provider::Icu => crate::icu::backfill::backfill(user_id, event_time, facade, store).await,
    }
}

/// The provider-agnostic `sync` dispatch (ADR-0014): enqueue a `Sync` event
/// on the user's provider queue. Delegates to the sync switch
/// ([`sync_user`](crate::dispatch::sync_user)). The admin trigger is
/// fire-and-forget, so the created gear ids are dropped.
pub async fn sync<S, D: EntityFacade<S>>(
    user_id: UserId,
    event_time: i64,
    facade: &D,
    store: &mut S,
) -> TbResult<()>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    sync_user(user_id, event_time, facade, store)
        .await
        .map(|_| ())
}

/// The provider-agnostic `deauthorize` dispatch (ADR-0014): revoke the
/// target's provider token and disable their local row (admin-only — the
/// driver verifies the acting session is an admin). The provider is the
/// target's real one, derived in-transaction from the provider-internal user
/// data.
pub async fn deauthorize<S>(session: &mut dyn Session, tbid: UserId, store: &mut S) -> TbResult<()>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    let provider = crate::provider_of(store, tbid).await?;
    match provider {
        Provider::Strava => crate::strava::user_deauthorize(session, tbid, store).await,
        Provider::Icu => crate::icu::user_deauthorize(session, tbid, store).await,
    }
}

/// The provider-agnostic `delete_user` dispatch (ADR-0014): deauthorize the
/// target (admin-only) plus delete their provider and domain rows. The
/// provider is the target's real one, read in-transaction.
pub async fn delete_user<S, D: EntityFacade<S>>(
    session: &mut dyn Session,
    tbid: UserId,
    facade: &D,
    store: &mut S,
) -> TbResult<()>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    let provider = crate::provider_of(store, tbid).await?;
    match provider {
        Provider::Strava => crate::strava::user_delete(session, tbid, facade, store).await,
        Provider::Icu => crate::icu::user_delete(session, tbid, facade, store).await,
    }
}

/// The provider-agnostic `activity_url` dispatch (ADR-0014): the provider's
/// URL for `id`, gated on the acting session's ownership (the read is the
/// ownership check). The driver extracts the activity id's prefix.
pub async fn activity_url<S, D: EntityFacade<S>>(
    provider: Provider,
    session: Box<dyn Session>,
    id: ActivityId,
    facade: &D,
    store: &mut S,
) -> TbResult<String>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    match provider {
        Provider::Strava => crate::strava::activity::strava_url(id, session, facade, store).await,
        Provider::Icu => crate::icu::activity::activity_url(id, session, facade, store).await,
    }
}

/// The provider-agnostic `gear_url` dispatch (ADR-0014): the provider's gear
/// page for the part `id`, gated on the acting session's ownership. The one
/// non-total op: icu gear is a private settings item with no public page, so
/// the icu arm is the typed absence (`None`) — the route maps it to a no-op.
pub async fn gear_url<S, D: EntityFacade<S>>(
    provider: Provider,
    session: Box<dyn Session>,
    id: PartId,
    facade: &D,
    store: &mut S,
) -> TbResult<Option<String>>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    match provider {
        Provider::Strava => {
            crate::strava::gear::strava_url(i32::from(id), session, facade, store).await
        }
        Provider::Icu => crate::icu::gear::gear_url(id, session, store).await,
    }
}

/// The provider-agnostic `user_url` dispatch (ADR-0014): the provider's
/// public athlete profile URL for the target `tbid` (no session: the route
/// has no user context to gate on). The icu arm builds the public athlete
/// profile page from the icu user row's athlete id. A pure data-store read:
/// it drives no domain work, so it takes no facade.
pub async fn user_url<S>(provider: Provider, id: UserId, store: &mut S) -> TbResult<String>
where
    S: crate::strava::StravaDataStore + crate::icu::IcuDataStore,
{
    match provider {
        Provider::Strava => crate::strava::strava_url(id, store).await,
        Provider::Icu => crate::icu::user::user_url(id, store).await,
    }
}

#[cfg(test)]
mod dispatch_tests;
