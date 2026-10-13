//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//! It is generic over the domain store (`St`) and the [`TxnSource`] (`source`),
//! which hands out one connection per open transaction: the web layer is
//! storage-agnostic and names only the [`TxnSource`] trait, never a concrete
//! store (ADR-0005, executable spec #446 §6). The concrete source (`tb_sqlx`'
//! `DbPool`) is injected at the composition root.
//!
//! The domain is the concrete one over the store (`ConcreteDomain<St>`,
//! spec #525 / ADR-0006): the web layer drives the concrete domain's entity
//! operations directly, so it is generic over the store, not over an
//! arbitrary [`tb_domain::Domain`].
//!
//! It also carries the per-user executor [`Registry`] (an `Arc`, so every clone of
//! the `AppState` — and thus every request — shares the one map of live executors,
//! spec #446 §4.4).

use std::sync::Arc;

use tb_domain::{ConcreteDomain, Store};
use tb_exec::TxnSource;

use crate::stream::{HEARTBEAT_INTERVAL, IDLE_TIMEOUT, Registry};

/// `Send + Sync` on `S` come from [`TxnSource`].
pub struct AppState<St: Store + 'static, S: TxnSource<ConcreteDomain<St>> + Clone + 'static> {
    pub(crate) source: S,
    pub(crate) registry: Arc<Registry<St>>,
}

// Hand-written (not derived): the derive would bound `St` on `Clone`, and the
// store is not `Clone` — every field clones without it (the `Registry` is
// shared through its `Arc`).
impl<St: Store + 'static, S: TxnSource<ConcreteDomain<St>> + Clone + 'static> Clone
    for AppState<St, S>
{
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            registry: Arc::clone(&self.registry),
        }
    }
}

impl<St: Store + 'static, S: TxnSource<ConcreteDomain<St>> + Clone + 'static> AppState<St, S> {
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
    pub fn with_registry(source: S, registry: Arc<Registry<St>>) -> Self {
        Self { source, registry }
    }
}
