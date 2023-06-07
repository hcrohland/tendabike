//! This module contains the definition of the `AppState` struct and its implementations.
//!
//! The `AppState` struct holds the application state that is shared across all requests.
//! It contains a `MemoryStore` for session management, a `StravaClient` for OAuth authentication,
//! and a `DbPool` for database connections.
//!
//! The `FromRef` trait is implemented for `MemoryStore`, `StravaClient`, and `PooledConnection<ConnectionManager<PgConnection>>`
//! to allow easy extraction of these components from a reference to `AppState`.


use async_session::MemoryStore;
use axum_macros::FromRef;

use crate::{strava::StravaClient, DbPool};

#[derive(Clone, FromRef)]
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

