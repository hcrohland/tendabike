//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//! It is generic over the [`TxnSource`] (`source`), which hands out one connection per
//! open transaction: the web layer is storage-agnostic and names only the [`TxnSource`]
//! trait, never a concrete store (ADR-0005, executable spec #446 §6). The concrete
//! source (`tb_sqlx`'s `DbPool`) is injected at the composition root.
//!
//! It also carries the per-user executor [`Registry`] (an `Arc`, so every clone of
//! the `AppState` — and thus every request — shares the one map of live executors,
//! spec #446 §4.4).

use std::sync::Arc;

use tb_exec::TxnSource;

use crate::stream::{HEARTBEAT_INTERVAL, IDLE_TIMEOUT, Registry};

/// `Send + Sync` on `S` come from [`TxnSource`].
#[derive(Clone)]
pub struct AppState<S: TxnSource + Clone + 'static> {
    pub(crate) source: S,
    pub(crate) registry: Arc<Registry>,
}

impl<S: TxnSource + Clone + 'static> AppState<S> {
    /// Build the state with the production [`Registry`] defaults (the 60s idle
    /// reap window and the 15s SSE heartbeat).
    pub fn new(source: S) -> Self {
        Self::with_registry(
            source,
            Arc::new(Registry::new(IDLE_TIMEOUT, HEARTBEAT_INTERVAL)),
        )
    }

    /// Build the state with an explicit [`Registry`] (the tests use a short
    /// idle/heartbeat cadence so the executor's lifecycle is observable).
    pub fn with_registry(source: S, registry: Arc<Registry>) -> Self {
        Self { source, registry }
    }
}
