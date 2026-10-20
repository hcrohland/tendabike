//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//!
//! It is non-generic (ADR-0007, executable spec #558 §7): it carries the
//! app's service API as `Arc<dyn ServiceApi>` — the exec-owned surface the
//! handlers call (reads, writes, the hydration write, the stream
//! subscription, the admin stats read, and the direct ops). The concrete
//! adapter (the `tb_exec::Registry` over the database pool) is built at the
//! composition root; the web layer names the trait, never a seam type.

use std::sync::Arc;

use tb_exec::ServiceApi;

#[derive(Clone)]
pub struct AppState {
    pub(crate) api: Arc<dyn ServiceApi>,
}

impl AppState {
    /// Build the state over the service API handle (the composition root
    /// builds the `Registry` over the pool and hands it over as
    /// `Arc<dyn ServiceApi>`).
    pub fn new(api: Arc<dyn ServiceApi>) -> Self {
        Self { api }
    }
}
