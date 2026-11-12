//! The per-user provider lane: the [`ProviderLane`] impl the executor loop
//! runs in production.
//!
//! The loop is generic over a single [`ProviderLane`] type, but a user has
//! exactly one provider (the `users.provider` column, immutable). This lane
//! wraps both provider lanes — [`StravaLane`] and [`IcuLane`] — and selects
//! between them per call from the user's `provider` column, so the loop's
//! control flow (the biased select, the backoff, the idle policy, the frame
//! push) is unchanged and provider-agnostic.
//!
//! The unified event type ([`LaneEvent`]) is the loop's `P::Event`: an
//! enum over the two providers' event rows, so the loop's `Select`, the
//! consumed-event channel, and the `run_event` dispatch are all
//! provider-agnostic.

use tb_domain::{ActivityId, PartId, Session, Summary, TbResult, UserId, UserStat};
use tb_provider::icu::{IcuLane, IcuStore, event::IcuEvent};
use tb_provider::strava::{StravaLane, StravaStore, event::Event as StravaEvent};
use tb_provider::{Probe, ProviderLane};

/// The unified provider event the per-user lane reports: the two providers'
/// event rows under one type, so the loop's `P::Event` is provider-agnostic
/// (spec #764: the `Message` enum's `Strava`/`Icu` arms).
#[derive(Debug, Clone, PartialEq)]
pub enum LaneEvent {
    /// A Strava event row (the `strava_events` queue).
    Strava(StravaEvent),
    /// An intervals.icu event row (the `icu_events` queue).
    Icu(IcuEvent),
}

/// A Strava event is a `LaneEvent`; a non-Strava one is `None` (the admin
/// `sync` op is Strava-only, so it correlates only the Strava arm).
impl From<LaneEvent> for Option<StravaEvent> {
    fn from(event: LaneEvent) -> Self {
        match event {
            LaneEvent::Strava(event) => Some(event),
            LaneEvent::Icu(_) => None,
        }
    }
}

/// The per-user provider lane: wraps [`StravaLane`] and [`IcuLane`] and
/// selects between them per call from the user's `provider` column (spec
/// #764). The loop is generic over this single type; it never names a
/// concrete provider.
///
/// Generic over the store type `S` (the transaction's connection):
/// `type Store = S`, so the loop's transaction connection ties to `S` and
/// `probe`/`process` operate on it. `S` must be a complete store for both
/// providers (`StravaStore + IcuStore`), which also carries the domain
/// `Store` (the `user_get` the per-call selection reads).
///
/// The marker-field impls (`Clone`/`Copy`/`Debug`/`Default`) are written by
/// hand rather than derived: both sub-lanes hold no `S` data (only a
/// `PhantomData<S>` marker) and are `Clone`/`Copy`/`Debug`/`Default` for any
/// `S`, so the wrapper is too — including for a database connection, which is
/// neither `Clone` nor `Debug`.
pub struct Lane<S: StravaStore + IcuStore> {
    strava: StravaLane<S>,
    icu: IcuLane<S>,
}

impl<S: StravaStore + IcuStore> Clone for Lane<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: StravaStore + IcuStore> Copy for Lane<S> {}

impl<S: StravaStore + IcuStore> std::fmt::Debug for Lane<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Lane").finish()
    }
}

impl<S: StravaStore + IcuStore> Default for Lane<S> {
    fn default() -> Self {
        Self {
            strava: StravaLane::default(),
            icu: IcuLane::default(),
        }
    }
}

// `Lane<S>` holds no `S` data (both sub-lanes are `PhantomData<S>` markers),
// so it is safe to share across threads even when `S` is not `Sync`. The
// auto-derived `Sync` would require `S: Sync`; this sound `unsafe impl`
// provides the unconditional `Sync` the `ProviderLane: Send + Sync` bound
// needs.
unsafe impl<S: StravaStore + IcuStore> Sync for Lane<S> {}

impl<S: StravaStore + IcuStore> Lane<S> {
    /// The per-user lane over a store of type `S` (both provider lanes).
    pub fn new() -> Self {
        Self {
            strava: StravaLane::new(),
            icu: IcuLane::new(),
        }
    }

    /// The user's provider (`'strava'` | `'icu'`), read from the
    /// `users.provider` column (through the owning entity's
    /// [`UserId::provider`] read, ADR-0010). A read failure is a DB failure
    /// the caller classifies.
    async fn provider_of(store: &mut S, user_id: UserId) -> TbResult<String> {
        user_id.provider(store).await
    }
}

#[async_trait::async_trait]
impl<S> ProviderLane for Lane<S>
where
    S: StravaStore + IcuStore + Send + 'static,
{
    type Event = LaneEvent;
    type Store = S;

    /// Probe the user's provider event queue (one transaction's
    /// housekeeping): read the user's `provider` column, then delegate to
    /// that provider's lane. The returned event is wrapped in [`LaneEvent`].
    async fn probe(&self, user_id: UserId, store: &mut S) -> Probe<LaneEvent> {
        let provider = match Self::provider_of(store, user_id).await {
            Ok(provider) => provider,
            Err(_) => return Probe::Error,
        };
        if provider == "icu" {
            match self.icu.probe(user_id, store).await {
                Probe::Event(event) => Probe::Event(LaneEvent::Icu(event)),
                Probe::Limited(until) => Probe::Limited(until),
                Probe::Empty => Probe::Empty,
                Probe::Error => Probe::Error,
            }
        } else {
            match self.strava.probe(user_id, store).await {
                Probe::Event(event) => Probe::Event(LaneEvent::Strava(event)),
                Probe::Limited(until) => Probe::Limited(until),
                Probe::Empty => Probe::Empty,
                Probe::Error => Probe::Error,
            }
        }
    }

    /// Process the user's next queued event (the work transaction): read the
    /// user's `provider` column, then delegate to that provider's lane.
    async fn process(&self, user_id: UserId, store: &mut S) -> Option<Summary> {
        let provider = match Self::provider_of(store, user_id).await {
            Ok(provider) => provider,
            Err(_) => return None,
        };
        if provider == "icu" {
            self.icu.process(user_id, store).await
        } else {
            self.strava.process(user_id, store).await
        }
    }

    /// Queue a sync event on the user's provider event queue (the onboarding
    /// cutover's provider side): delegate to the user's provider's lane.
    async fn insert_sync(
        &self,
        user_id: UserId,
        time: i64,
        auto: bool,
        store: &mut S,
    ) -> TbResult<()> {
        let provider = Self::provider_of(store, user_id).await?;
        if provider == "icu" {
            self.icu.insert_sync(user_id, time, auto, store).await
        } else {
            self.strava.insert_sync(user_id, time, auto, store).await
        }
    }

    /// Import the user's gear from their provider (the hydration write's
    /// provider side): delegate to the user's provider's lane.
    async fn update_gear(&self, user_id: UserId, store: &mut S) -> TbResult<Vec<PartId>> {
        let provider = Self::provider_of(store, user_id).await?;
        if provider == "icu" {
            self.icu.update_gear(user_id, store).await
        } else {
            self.strava.update_gear(user_id, store).await
        }
    }

    /// Read every user's provider-neutral stat (the admin read's provider
    /// side): the union of the Strava users' stats and the icu users' stats
    /// (each user's stat is sourced from their own provider).
    async fn all_stats(&self, store: &mut S) -> TbResult<Vec<UserStat>> {
        let mut stats = self.strava.all_stats(store).await?;
        stats.extend(self.icu.all_stats(store).await?);
        Ok(stats)
    }

    /// The per-user half of [`ProviderLane::activity_url`]: read the user's
    /// `provider` column, then delegate to that provider's lane.
    async fn activity_url(
        &self,
        id: ActivityId,
        session: Box<dyn Session>,
        store: &mut S,
    ) -> TbResult<String> {
        let provider = Self::provider_of(store, session.user_id()).await?;
        if provider == "icu" {
            self.icu.activity_url(id, session, store).await
        } else {
            self.strava.activity_url(id, session, store).await
        }
    }
}
