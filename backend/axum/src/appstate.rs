//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//! It is generic over the [`TxnSource`] (`source`), which hands out one connection per
//! open transaction: the web layer is storage-agnostic and names only the [`TxnSource`]
//! trait, never a concrete store (ADR-0005, executable spec #446 §6). The concrete
//! source (`tb_sqlx`'s `DbPool`) is injected at the composition root.
//!
//! Handlers extract `State<AppState<S>>` (axum's reflexive `FromRef` impl) and reach the
//! source through the `source` field.

use tb_exec::TxnSource;

/// `Send + Sync` on `S` come from [`TxnSource`].
#[derive(Clone)]
pub struct AppState<S: TxnSource + Clone + 'static> {
    pub(crate) source: S,
}

impl<S: TxnSource + Clone + 'static> AppState<S> {
    pub fn new(source: S) -> Self {
        Self { source }
    }
}
