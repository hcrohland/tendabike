//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//!
//! It is non-generic (ADR-0007, executable spec #558 §7): it carries the
//! app's service API as `Arc<dyn ServiceApi>` (the generic ops — reads,
//! writes, the hydration write, the stream subscription, the admin stats
//! read — plus the provider-neutral admin ops, `rescan`, the shop-stream
//! subscription, and the batched `trigger_processing`) and the
//! provider-agnostic provider service as `Arc<dyn ProviderService>` (the
//! ADR-0014 direct ops — the user-keyed `sync`/`deauthorize`/`delete_user`
//! that dispatch on the user's real provider, and the redirect URLs). The
//! per-provider handles (the Strava-specific `ProviderService` and the
//! `IcuService`) are the old surface, intact until their routes migrate. The
//! concrete adapter (the `tb_exec::Registry` over the database pool) is
//! built at the composition root; the web layer names the traits, never a
//! seam type.

use std::sync::Arc;

use tb_exec::{IcuService, ProviderService, ServiceApi};

#[derive(Clone)]
pub(crate) struct AppState {
    /// The generic service API (reads, writes, hydration, the stream
    /// subscription, the admin stats read, and the provider-neutral admin
    /// ops — `rescan`, the shop-stream subscription, the batched
    /// `trigger_processing`).
    pub(crate) api: Arc<dyn ServiceApi>,
    /// The provider-agnostic provider service (ADR-0014): the direct ops the
    /// neutral routes drive the provider through. The user-keyed ops
    /// (`sync`, `deauthorize`, `delete_user`) dispatch on the user's real
    /// provider, read in-transaction — a per-provider route acting on a user
    /// of the other provider follows the stated identity rule.
    pub(crate) provider: Arc<dyn tb_exec::providerservice::ProviderService>,
    /// The Strava-specific service API (OAuth, webhook, the redirect URLs) —
    /// the old surface, intact until its routes migrate (ADR-0014).
    pub(crate) strava: Arc<dyn ProviderService>,
    /// The icu-specific service API (OAuth, webhook) — the icu peer of
    /// [`ProviderService`] (spec #763).
    pub(crate) icu: Arc<dyn IcuService>,
}

impl AppState {
    /// Build the state over the service API and provider service handles
    /// (the composition root builds the `Registry` over the pool and hands
    /// it over as the four `Arc<dyn _>` handles).
    pub(crate) fn new(
        api: Arc<dyn ServiceApi>,
        provider: Arc<dyn tb_exec::providerservice::ProviderService>,
        strava: Arc<dyn ProviderService>,
        icu: Arc<dyn IcuService>,
    ) -> Self {
        Self {
            api,
            provider,
            strava,
            icu,
        }
    }
}
