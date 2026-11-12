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
//! ADR-0014 direct ops — the login and the webhook intake, the user-keyed
//! `sync`/`deauthorize`/`delete_user` that dispatch on the user's real
//! provider, and the redirect URLs). The web state is two handles, so a new
//! provider never reshapes it (ADR-0014). The concrete adapter (the
//! `tb_exec::Registry` over the database pool) is built at the composition
//! root; the web layer names the traits, never a seam type.

use std::sync::Arc;

use tb_exec::{ServiceApi, providerservice::ProviderService};

#[derive(Clone)]
pub(crate) struct AppState {
    /// The generic service API (reads, writes, hydration, the stream
    /// subscription, the admin stats read, and the provider-neutral admin
    /// ops — `rescan`, the shop-stream subscription, the batched
    /// `trigger_processing`).
    pub(crate) api: Arc<dyn ServiceApi>,
    /// The provider-agnostic provider service (ADR-0014): the direct ops the
    /// routes drive the provider through. The route-keyed ops (`login`,
    /// `accept_event`) take the typed `Provider` (the route knows it from
    /// the URL) plus opaque string input; the user-keyed ops
    /// (`sync`, `deauthorize`, `delete_user`) dispatch on the user's real
    /// provider, read in-transaction — a per-provider route acting on a user
    /// of the other provider follows the stated identity rule.
    pub(crate) provider: Arc<dyn ProviderService>,
}

impl AppState {
    /// Build the state over the service API and provider service handles
    /// (the composition root builds the `Registry` over the pool and hands
    /// it over as the two `Arc<dyn _>` handles).
    pub(crate) fn new(api: Arc<dyn ServiceApi>, provider: Arc<dyn ProviderService>) -> Self {
        Self { api, provider }
    }
}
