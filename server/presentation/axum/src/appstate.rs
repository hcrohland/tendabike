//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//! It contains a `MemoryStore` for session management, a `StravaClient` for OAuth authentication,
//! and a `DbPool` for database connections.
//!
//! The `FromRef` trait is implemented for `MemoryStore`, `StravaClient`, and `PooledConnection<ConnectionManager<PgConnection>>`
//! to allow easy extraction of these components from a reference to `AppState`.

use async_session::MemoryStore;
use axum::extract::FromRef;
use diesel::{r2d2::ConnectionManager, PgConnection};
use r2d2::PooledConnection;

use crate::{strava::StravaClient, DbPool};

#[derive(Clone)]
pub(super) struct AppState {
    store: MemoryStore,
    oauth_client: StravaClient,
    pool: DbPool,
}

impl AppState {
    pub(super) fn new(store: MemoryStore, oauth_client: StravaClient, pool: DbPool) -> Self {
        Self {
            store,
            oauth_client,
            pool,
        }
    }
}

impl FromRef<AppState> for MemoryStore {
    fn from_ref(state: &AppState) -> Self {
        state.store.clone()
    }
}

impl FromRef<AppState> for StravaClient {
    fn from_ref(state: &AppState) -> Self {
        state.oauth_client.clone()
    }
}

impl FromRef<AppState> for PooledConnection<ConnectionManager<PgConnection>> {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone().get().unwrap()
    }
}
