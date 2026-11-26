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
//! forwards to. Each is a typed switch on [`Provider`] that drives the
//! [`Provider`](crate::driver::Provider) driver for the selected marker —
//! the single home for the provider knowledge on the direct-op path, a
//! sibling of the unified lane seam (`unified`). The executor is a thin
//! forward: it opens the transaction, hands the connection (and the
//! entity-op facade) to one of these, and commits; no `match provider` and
//! no driver name appear in `tb_exec`.
//!
//! The target-keyed ops (`sync`, `deauthorize`, `delete_user`, `gear_url`,
//! `user_url`) dispatch on the target's real provider, derived in-transaction
//! (the [`provider_of`](crate::provider_of) lookup, ADR-0014), not on the
//! route: a per-provider route acting on a target of the other provider
//! follows the stated identity rule instead of failing on a missing lookup.
//!
//! The store is the [`ProviderStore`](crate::ProviderStore) supertrait — the
//! single home for the "all provider stores" bound: each arm drives its own
//! driver's [`Provider`](crate::driver::Provider) interface through the
//! per-provider [`Store`](crate::store::Store) marker the supertrait
//! composes, and the switch is exhaustive over the enum. Domain work goes
//! through the entity-op facade (a parameter — the registry passes the
//! concrete one); the pure queue-intake ops (`accept_event`, `user_url`)
//! need no facade.
//!
//! [`ProviderService`]: crate::ProviderService

use log::{info, warn};

use crate::facade::{ActivityId, Error, PartId, Session, TbResult, UserId, UserStat};

use crate::EntityFacade;
use crate::Provider;
use crate::ProviderActivityId;
use crate::ProviderSpec;
use crate::ProviderStore;
use crate::driver::Provider as Driver;
use crate::icu::{IcuDriver, IcuSpec};
use crate::store::Store;
use crate::strava::{StravaDriver, StravaSpec};

/// The provider-agnostic `login` dispatch (ADR-0014): the provider's login
/// work — the scope check, the token exchange, the athlete wire-shape
/// parsing, and the upsert — returning the identity and the admin flag. The
/// provider is the route's (it knows it from the URL); `code`, `scope`, and
/// `verifier` are the opaque callback values (no driver wire type crosses the
/// seam). `verifier` is the PKCE code verifier (intervals.icu-only — the
/// Strava flow is a plain authorization-code exchange and ignores it); the
/// icu arm requires it (the web layer always recovers it from the signed
/// state, so its absence is a protocol violation).
pub async fn login<S, D: EntityFacade<S>>(
    provider: Provider,
    code: &str,
    scope: &str,
    verifier: Option<&str>,
    facade: &D,
    store: &mut S,
) -> TbResult<(UserId, bool)>
where
    S: ProviderStore,
{
    // A disabled provider is a graceful freeze (ADR-0016): no OAuth without
    // credentials — a clear "unavailable", not a token exchange.
    crate::config::require_enabled(provider)?;
    match provider {
        Provider::Strava => StravaDriver::login(code, scope, None, facade, store).await,
        Provider::Icu => IcuDriver::login(code, scope, verifier, facade, store).await,
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
    S: ProviderStore,
{
    // A disabled provider's webhook is declined (ADR-0016): the clear
    // `Unavailable` error answers a non-2xx, so the sender retries and
    // the event is not lost — re-adding the credential restores the
    // provider and its syncs.
    crate::config::require_enabled(provider)?;
    match provider {
        Provider::Strava => StravaDriver::accept_event(body, store).await,
        Provider::Icu => IcuDriver::accept_event(body, store).await,
    }
}

/// The per-user onboarding sync (ADR-0014): the single home for the user's
/// provider identity read plus the per-driver `onboard` arms. The provider is
/// the user's real one, read in-transaction (a per-provider route acting on a
/// user of the other provider follows the stated identity rule). Each driver's
/// `onboard` primitive imports the user's gear (bikes + shoes) and enqueues
/// the activity sync, returning the created gear part ids (spec #829) — the
/// combined gear-import-plus-queue sync op. The admin trigger delegates here
/// (the registry's
/// [`sync`](crate::dispatch::sync)); it drops the gear ids.
pub(crate) async fn sync_user<S, D: EntityFacade<S>>(
    user_id: UserId,
    event_time: i64,
    facade: &D,
    store: &mut S,
) -> TbResult<Vec<PartId>>
where
    S: ProviderStore,
{
    let provider = crate::provider_of(store, user_id).await?;
    // A disabled provider's sync fails loudly (ADR-0016): the graceful
    // freeze is a clear "provider disabled" error, not a silent no-op.
    crate::config::require_enabled(provider)?;
    match provider {
        Provider::Strava => StravaDriver::onboard(user_id, event_time, facade, store).await,
        Provider::Icu => IcuDriver::onboard(user_id, event_time, facade, store).await,
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
    S: ProviderStore,
{
    sync_user(user_id, event_time, facade, store)
        .await
        .map(|_| ())
}

/// The provider-agnostic `trigger_processing` dispatch (spec #855): a pure
/// activity re-sync — queue a sync for every non-disabled user on their own
/// provider's lane, without re-importing gear (gear import is an
/// onboarding-only concern). A Strava user gets a Strava `Sync` event via
/// `insert_sync`; an icu user gets the bare `SYNC` event. `user_id` narrows
/// the trigger to one user, regardless of provider (a user with no row in
/// either table is a `NotFound`). Returns the tbids of the users a sync was
/// queued for — the caller's per-user wake list.
pub async fn sync_users<S>(
    session: &mut dyn Session,
    user_id: Option<UserId>,
    time: i64,
    store: &mut S,
) -> TbResult<Vec<UserId>>
where
    S: ProviderStore,
{
    if !session.is_admin() {
        return Err(Error::Forbidden(
            "only an admin can trigger processing".to_string(),
        ));
    }

    // Which lanes to run: both for the all-users trigger, the one the target
    // user belongs to for a single-user trigger (the `provider_of` lookup).
    let provider = match user_id {
        Some(id) => Some(crate::provider_of(store, id).await?),
        None => None,
    };

    let mut queued = Vec::new();

    // The Strava lane: a `Sync` event per non-disabled Strava user. A
    // disabled provider's lane is skipped (ADR-0016: the admin sync skips
    // its users — their syncs would fail with the clear error anyway).
    if (provider.is_none() || provider == Some(Provider::Strava))
        && crate::enabled_providers().contains(&Provider::Strava)
    {
        let users = match user_id {
            Some(id) => vec![Store::<StravaSpec>::user_get_by_tbid(store, id).await?],
            None => Store::<StravaSpec>::user_get_all(store).await?,
        };
        for user in users {
            if StravaDriver::disabled(&user) {
                warn!("user {} disabled, skipping", StravaSpec::user_owner(&user));
                continue;
            }
            info!(
                "Adding sync for {:?} at {time}",
                StravaSpec::user_tbid(&user)
            );
            crate::strava::event::insert_sync(*StravaSpec::user_tbid(&user), time, store).await?;
            queued.push(*StravaSpec::user_tbid(&user));
        }
    }

    // The icu lane: a `SYNC` event (the durable backfill cursor) per
    // non-disabled icu user — a pure activity re-sync, no gear import (gear
    // import is an onboarding-only concern). A disabled provider's lane is
    // skipped (ADR-0016).
    if (provider.is_none() || provider == Some(Provider::Icu))
        && crate::enabled_providers().contains(&Provider::Icu)
    {
        let users = match user_id {
            Some(id) => vec![Store::<IcuSpec>::user_get_by_tbid(store, id).await?],
            None => Store::<IcuSpec>::user_get_all(store).await?,
        };
        for user in users {
            if IcuDriver::disabled(&user) {
                warn!("user {} disabled, skipping", IcuSpec::user_tbid(&user));
                continue;
            }
            info!("Adding sync for {:?} at {time}", IcuSpec::user_tbid(&user));
            crate::icu::event::insert_sync(*IcuSpec::user_tbid(&user), time, store).await?;
            queued.push(*IcuSpec::user_tbid(&user));
        }
    }

    Ok(queued)
}

/// The provider-agnostic `all_stats` dispatch (spec #558 §3.3): every
/// enabled provider's users as provider-neutral [`UserStat`]s — each row
/// stamped with its provider's wire value by the driver's `all_stats`. The
/// admin's list is provider-agnostic (an admin of one provider sees the
/// other provider's users too), so the two lanes are driven in one
/// transaction and the results concatenated, in the `Provider` enum's
/// definition order. A disabled provider's lane is skipped (ADR-0016: its
/// users are frozen on this server, not listed).
pub async fn all_stats<S, D: EntityFacade<S>>(facade: &D, store: &mut S) -> TbResult<Vec<UserStat>>
where
    S: ProviderStore,
{
    let mut stats = Vec::new();

    // The Strava lane: a disabled provider's lane is skipped (ADR-0016).
    if crate::enabled_providers().contains(&Provider::Strava) {
        stats.extend(StravaDriver::all_stats(facade, store).await?);
    }

    // The icu lane: a disabled provider's lane is skipped (ADR-0016).
    if crate::enabled_providers().contains(&Provider::Icu) {
        stats.extend(IcuDriver::all_stats(facade, store).await?);
    }

    Ok(stats)
}

/// The provider-agnostic `deauthorize` dispatch (ADR-0014): revoke the
/// target's provider token and disable their local row (admin-only — the
/// driver verifies the acting session is an admin). The provider is the
/// target's real one, derived in-transaction from the provider-internal user
/// data.
pub async fn deauthorize<S>(session: &mut dyn Session, tbid: UserId, store: &mut S) -> TbResult<()>
where
    S: ProviderStore,
{
    let provider = crate::provider_of(store, tbid).await?;
    match provider {
        Provider::Strava => StravaDriver::user_deauthorize(session, tbid, store).await,
        Provider::Icu => IcuDriver::user_deauthorize(session, tbid, store).await,
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
    S: ProviderStore,
{
    let provider = crate::provider_of(store, tbid).await?;
    match provider {
        Provider::Strava => StravaDriver::user_delete(session, tbid, facade, store).await,
        Provider::Icu => IcuDriver::user_delete(session, tbid, facade, store).await,
    }
}

/// The provider-agnostic `activity_url` dispatch (ADR-0014): the provider's
/// URL for `id`, gated on the acting session's ownership (the read is the
/// ownership check). The provider is the activity id's own — parsed from its
/// prefix (the single home for the prefix knowledge, ADR-0013), not the route.
pub async fn activity_url<S, D: EntityFacade<S>>(
    session: Box<dyn Session>,
    id: ActivityId,
    facade: &D,
    store: &mut S,
) -> TbResult<String>
where
    S: ProviderStore,
{
    let provider = Provider::try_from(ProviderActivityId::try_from(id.clone())?)?;
    match provider {
        Provider::Strava => StravaDriver::activity_url(session, id, facade, store).await,
        Provider::Icu => IcuDriver::activity_url(session, id, facade, store).await,
    }
}

/// The provider-agnostic `gear_url` dispatch (ADR-0014): the provider's gear
/// page for the part `id`, gated on the acting session's ownership. The
/// provider is the part's real one, derived in-transaction (the
/// [`provider_of`](crate::provider_of) lookup). The one non-total op: icu
/// gear is a private settings item with no public page, so the icu arm is the
/// typed absence (`None`) — the route maps it to a no-op.
pub async fn gear_url<S, D: EntityFacade<S>>(
    session: Box<dyn Session>,
    id: PartId,
    facade: &D,
    store: &mut S,
) -> TbResult<Option<String>>
where
    S: ProviderStore,
{
    // The part read is the ownership check; the provider is the part
    // owner's real one (the domain stores no provider). The arms drive the
    // driver's `gear_url` from the read part: the Strava arm's source, the
    // icu arm the typed absence (icu gear has no public page).
    let part = facade.part(id, session, store).await?;
    let provider = crate::provider_of(store, part.owner).await?;
    match provider {
        Provider::Strava => StravaDriver::gear_url(&part),
        Provider::Icu => IcuDriver::gear_url(&part),
    }
}

/// The provider-agnostic `user_url` dispatch (ADR-0014): the provider's
/// public athlete profile URL for the target `tbid` (no session: the route
/// has no user context to gate on). The provider is the target's real one,
/// derived in-transaction (the [`provider_of`](crate::provider_of) lookup).
/// The icu arm builds the public athlete profile page from the icu user
/// row's athlete id. A pure data-store read: it drives no domain work, so it
/// takes no facade.
pub async fn user_url<S>(id: UserId, store: &mut S) -> TbResult<String>
where
    S: ProviderStore,
{
    let provider = crate::provider_of(store, id).await?;
    match provider {
        Provider::Strava => StravaDriver::user_url(id, store).await,
        Provider::Icu => IcuDriver::user_url(id, store).await,
    }
}

#[cfg(test)]
mod dispatch_tests;
