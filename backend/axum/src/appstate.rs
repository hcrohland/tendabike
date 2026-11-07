//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//!
//! It is non-generic (ADR-0007, executable spec #558 §7): it carries the
//! app's service API as `Arc<dyn ServiceApi>` (the generic ops — reads,
//! writes, the hydration write, the stream subscription, the admin stats
//! read) and the provider-specific surface as `Arc<dyn ProviderService>`
//! (the direct ops — OAuth, webhook, sync, rescan, deauthorize, etc.). The
//! concrete adapter (the `tb_exec::Registry` over the database pool) is
//! built at the composition root; the web layer names the traits, never a
//! seam type.

use std::sync::Arc;

use tb_exec::{ProviderService, ServiceApi};

#[derive(Clone)]
pub struct AppState {
    /// The generic service API (reads, writes, hydration, subscription,
    /// stats).
    pub(crate) api: Arc<dyn ServiceApi>,
    /// The provider-specific service API (OAuth, webhook, sync, rescan,
    /// deauthorize, etc.).
    pub(crate) provider: Arc<dyn ProviderService>,
}

impl AppState {
    /// Build the state over the service API and provider service handles
    /// (the composition root builds the `Registry` over the pool and hands
    /// it over as `Arc<dyn ServiceApi>` + `Arc<dyn ProviderService>`).
    pub fn new(api: Arc<dyn ServiceApi>, provider: Arc<dyn ProviderService>) -> Self {
        Self { api, provider }
    }
}
