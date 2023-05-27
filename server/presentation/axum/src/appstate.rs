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
