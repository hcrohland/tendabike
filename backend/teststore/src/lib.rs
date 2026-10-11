//! The unified in-memory test store (spec #510).
//!
//! One store type — [`TestStore`] — implementing the nine `Store`
//! sub-traits, `StravaStore`, a self-referential [`TxnSource`]
//! (`begin()` returns a sibling sharing the same state), and [`Txn`]
//! (consuming, no-op commit/rollback). All state is one shared
//! [`TestStoreState`] held under `Arc<Mutex<…>>`, with no transaction
//! buffer: every write lands directly in the shared state, every read
//! sees it.
//!
//! The crate also owns the canonical fixture path (`prepopulated()`,
//! `snapshot()`, `build_workshop_store()`, the `fixtures::` builders,
//! and the `build-snapshot` bin) and the stream tests' panic seam
//! (`arm_panic()`).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tb_domain::{Store, TbResult, UserId};
use tb_exec::{Txn, TxnSource};
use tb_strava::StravaUser;
use tb_strava::event::Event;

mod prepopulated_data;
mod snapshot;
mod store_activity;
mod store_attachment;
mod store_data;
mod store_part;
mod store_service;
mod store_serviceplan;
mod store_shop;
mod store_usage;
mod store_user;
mod strava_store;

pub use snapshot::{StoreSnapshot, build_workshop_store};

pub use store_data::StoreData;
pub mod fixtures;

/// The unified in-memory test store (spec #510): one state, all traits
/// implemented directly, no delegation — structurally parallel to the
/// Postgres `SqlxConn`, with the transaction lifecycle reduced to a
/// no-op (there is no transaction buffer: every write lands directly in
/// the shared state, every read sees it).
#[derive(Clone)]
pub struct TestStore {
    /// The shared in-memory database, held for the executor's concurrent
    /// task + the test thread (axum stream tests); uniform for all users.
    state: Arc<Mutex<TestStoreState>>,
    /// The stream tests' panic seam (spec #446 §4.6).
    panic: Arc<AtomicBool>,
}

/// The complete state of the in-memory database: domain tables + the
/// Strava tables, plus all auto-increment counters.
#[derive(Clone)]
pub struct TestStoreState {
    /// The domain half (today's `StoreData`, moved here).
    pub domain: StoreData,
    /// The Strava half (today axum's `StravaState` + strava's user map).
    pub strava: StravaState,
}

/// In-memory stand-in for the `strava_events` queue and the
/// `strava_users` table. The refresh token lives on the `StravaUser`
/// row (`refresh_token: Option<RefreshToken>`), so there is no separate
/// token field (axum's `StravaState.refresh_token` is subsumed).
#[derive(Clone, Default)]
pub struct StravaState {
    pub events: Vec<Event>,
    pub next_event_id: i32,
    pub users: HashMap<UserId, StravaUser>,
}

impl Default for TestStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TestStore {
    /// An empty in-memory database (spec #510).
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TestStoreState {
                domain: StoreData::empty(),
                strava: StravaState::default(),
            })),
            panic: Arc::new(AtomicBool::new(false)),
        }
    }

    /// A clone of the shared state, under the lock — for post-write
    /// assertions and seeding (collapses today's `LiveSource::mem()`/
    /// `strava()` accessors into one public state surface).
    pub fn state(&self) -> TestStoreState {
        self.state
            .lock()
            .expect("test store state poisoned")
            .clone()
    }

    /// Seed a Strava user row directly (spec #510).
    pub fn insert_user(&mut self, user: StravaUser) {
        let mut state = self.state.lock().expect("test store state poisoned");
        state.strava.users.insert(user.tendabike_id, user);
    }

    /// The number of events in the Strava queue (assertion helper).
    pub fn event_count(&self) -> usize {
        let state = self.state.lock().expect("test store state poisoned");
        state.strava.events.len()
    }

    /// Arm the next Strava-queue read to panic (the stream tests' seam
    /// for the spec #446 §4.6 panic teardown: a panicked executor must
    /// record `Dead`, and the next write must respawn it). The check
    /// lives at the top of `strava_event_get_next_for_user`.
    pub fn arm_panic(&self) {
        self.panic.store(true, Ordering::SeqCst);
    }
}

/// All nine `Store` sub-traits are implemented directly on [`TestStore`]
/// (one state, no delegation — structurally parallel to the Postgres
/// `SqlxConn`, spec #510).
impl Store for TestStore {}

/// The self-referential `TxnSource` (spec #510): a sibling transaction is
/// a cheap `Arc` clone — the same shared state.
#[async_trait]
impl TxnSource for TestStore {
    type Conn = Self;

    async fn begin(&self) -> TbResult<Self::Conn> {
        Ok(self.clone())
    }
}

/// The consuming, no-op `Txn` (spec #510): there is no transaction buffer
/// to flush or discard, so commit and rollback just close the handle —
/// matching `SqlxConn`'s consuming lifecycle; the shared state lives on.
#[async_trait]
impl Txn for TestStore {
    async fn commit(self) -> TbResult<()> {
        Ok(())
    }

    async fn rollback(self) -> TbResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tb_domain::test_support::fixtures::test_session;
    use tb_domain::{PartStore, TbResult, UserId, UserStore};
    use tb_exec::{Txn, TxnSource};
    use tb_strava::event::Event;
    use tb_strava::{StravaId, StravaStore, StravaUser};

    /// `new()` starts from an empty database: no domain rows, no Strava
    /// users, no events (spec #510).
    #[test]
    fn new_starts_empty() {
        let store = TestStore::new();
        let state = store.state();
        assert!(state.domain.users.is_empty());
        assert!(state.domain.parts.is_empty());
        assert!(state.strava.users.is_empty());
        assert!(state.strava.events.is_empty());
    }

    /// `state()` hands back a clone of the shared state: mutating the
    /// clone must not touch the store (tests assert through it, they do
    /// not write through it).
    #[test]
    fn state_returns_a_clone() {
        let mut store = TestStore::new();
        store.insert_user(test_user());
        let mut snap = store.state();
        snap.strava.users.clear();
        assert_eq!(store.state().strava.users.len(), 1);
    }

    /// `insert_user` seeds a Strava user row, readable through
    /// `state()` (spec #510: tests seed through the two helpers, not a
    /// public raw mutator).
    #[test]
    fn insert_user_seeds_a_strava_user_row() {
        let mut store = TestStore::new();
        store.insert_user(test_user());
        let state = store.state();
        assert_eq!(state.strava.users.len(), 1);
        assert_eq!(state.strava.users[&UserId::from(1)].id, StravaId::from(42));
    }

    /// `event_count()` counts the Strava event queue.
    #[test]
    fn event_count_counts_the_queue() {
        let store = TestStore::new();
        assert_eq!(store.event_count(), 0);
    }

    /// `begin()` returns a sibling on the same shared state — the
    /// self-referential `TxnSource` (`type Conn = Self`, spec #510): a
    /// write through the sibling is visible on the original handle
    /// immediately, no commit in between.
    #[tokio::test]
    async fn begin_returns_a_sibling_sharing_the_same_state() -> TbResult<()> {
        let store = TestStore::new();
        let mut sibling = store.begin().await?;

        sibling.insert_user(test_user());

        assert_eq!(store.state().strava.users.len(), 1);
        Ok(())
    }

    /// `commit` is a no-op that consumes the handle (spec #510, matching
    /// `SqlxConn`'s consuming lifecycle): the shared state lives on in
    /// the source.
    #[tokio::test]
    async fn commit_is_a_noop_that_consumes_the_handle() -> TbResult<()> {
        let store = TestStore::new();
        let mut handle = store.begin().await?;

        handle.insert_user(test_user());
        handle.commit().await?;

        assert_eq!(store.state().strava.users.len(), 1);
        Ok(())
    }

    /// `rollback` is a no-op too: with no transaction buffer there is
    /// nothing to discard — the write is already in the shared state
    /// (spec #510, decided in #504).
    #[tokio::test]
    async fn rollback_is_a_noop_the_shared_state_lives_on() -> TbResult<()> {
        let store = TestStore::new();
        let mut handle = store.begin().await?;

        handle.insert_user(test_user());
        handle.rollback().await?;

        assert_eq!(store.state().strava.users.len(), 1);
        Ok(())
    }

    /// No transaction buffer (spec #510, decided in #504): a write made
    /// through one handle lands directly in the shared state — a sibling
    /// that began before the write sees it immediately, no commit in
    /// between.
    #[tokio::test]
    async fn writes_land_directly_in_the_shared_state() -> TbResult<()> {
        let mut store = TestStore::new();
        let mut sibling = store.begin().await?;

        store.create("Ada", "Lovelace", &None).await?;

        let user = sibling.get(UserId::from(1)).await?;
        assert_eq!(user.firstname, "Ada");
        Ok(())
    }

    /// `arm_panic()` arms the next Strava-queue read to panic — the
    /// stream tests' teardown seam (spec #446 §4.6, spec #510): the check
    /// lives at the top of `strava_event_get_next_for_user`.
    #[tokio::test]
    #[should_panic]
    async fn armed_panic_fires_on_the_next_strava_queue_read() {
        let mut store = TestStore::new();
        store.arm_panic();
        let _ = store
            .strava_event_get_next_for_user(StravaId::from(1))
            .await
            .unwrap();
    }

    /// The arm is one-shot: the panic clears it, and the next read on the
    /// shared state succeeds.
    #[tokio::test]
    async fn panic_arm_fires_once() -> TbResult<()> {
        let mut store = TestStore::new();
        store.arm_panic();
        let mut armed = store.clone();

        // The armed read must panic in its own runtime: a panic cannot be
        // unwound across this task's runtime boundary.
        let panicked = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(armed.strava_event_get_next_for_user(StravaId::from(1)))
                .unwrap()
        })
        .join();
        assert!(panicked.is_err(), "the armed read must panic");

        let next = store
            .strava_event_get_next_for_user(StravaId::from(1))
            .await?;
        assert!(next.is_none());
        Ok(())
    }

    /// The `StravaStore` methods operate on the shared Strava state
    /// (moved from `TestStravaStore`, spec #510): user lookups by both
    /// id directions, the event queue with assigned ids, and the token
    /// update.
    #[tokio::test]
    async fn strava_store_roundtrip() -> TbResult<()> {
        let mut store = TestStore::new();
        store.insert_user(test_user());

        // User lookups in both directions.
        assert_eq!(store.stravaid_get_user_id(1).await?, 42);
        assert!(store.stravaid_get_user_id(2).await.is_err());
        let by_tbid = store.stravauser_get_by_tbid(UserId::from(1)).await?;
        assert_eq!(by_tbid.id, StravaId::from(42));
        let by_stravaid = store
            .stravauser_get_by_stravaid(&StravaId::from(42))
            .await?
            .expect("user 42 is registered");
        assert_eq!(by_stravaid.tendabike_id, UserId::from(1));

        // The event queue: stored events get an assigned id, and the next
        // event for the owner is the earliest one.
        let event = Event {
            owner_id: StravaId::from(42),
            event_time: 1_000,
            ..Default::default()
        };
        store.stravaevent_store(event).await?;
        assert_eq!(store.event_count(), 1);
        let next = store
            .strava_event_get_next_for_user(StravaId::from(42))
            .await?
            .expect("one event in the queue");
        assert_eq!(next.id, Some(1));

        // The token update lands on the shared user row.
        let updated = store
            .stravaid_update_token(StravaId::from(42), Some(&"new-token".to_string()))
            .await?;
        assert!(updated.refresh_token.is_some());
        assert_eq!(
            store
                .state()
                .strava
                .users
                .get(&UserId::from(1))
                .expect("user row")
                .refresh_token
                .is_some(),
            true
        );
        Ok(())
    }

    /// Compile-time proof: `TestStore` satisfies the executor loop's
    /// bounds — `T: TxnSource + Clone + Send + Sync + 'static,
    /// T::Conn: StravaStore` (spec #510: the loop is generic over the
    /// source and names only the traits).
    #[allow(dead_code)]
    async fn executor_bounds<T>(source: T)
    where
        T: TxnSource + Clone + Send + Sync + 'static,
        T::Conn: StravaStore,
    {
        let _ = source.begin().await;
    }

    #[tokio::test]
    async fn test_store_satisfies_the_executor_bounds() {
        let store = TestStore::new();
        let _ = executor_bounds(store);
    }

    /// `prepopulated()` loads the canonical workshop snapshot into a
    /// fresh state, id counters derived from the loaded data (spec #510,
    /// moved from `MemStore::prepopulated`).
    #[test]
    fn prepopulated_loads_the_snapshot() {
        let store = TestStore::prepopulated();
        let state = store.state();
        assert_eq!(state.domain.users.len(), 1);
        assert_eq!(state.domain.parts.len(), 17);
        assert_eq!(state.domain.attachments.len(), 11);
        assert_eq!(state.domain.usages.len(), 11);
        assert_eq!(state.domain.activities.len(), 3);
        assert_eq!(state.domain.next_part_id, 18);
        assert_eq!(state.domain.next_user_id, 2);
        // No shops in the snapshot: the shop counter stays at its empty
        // start.
        assert_eq!(state.domain.next_shop_id, 1);
    }

    /// `snapshot()` is a deterministic export: exporting a
    /// `prepopulated()` store round-trips the snapshot JSON exactly
    /// (spec #510: the sqlx scratch seeder and the `build-snapshot` bin
    /// rely on this).
    #[test]
    fn snapshot_roundtrips_the_prepopulated_data() {
        let expected: StoreSnapshot = serde_json::from_str(prepopulated_data::SNAPSHOT_JSON)
            .expect("SNAPSHOT_JSON must be valid JSON");
        let actual = TestStore::prepopulated().snapshot();
        assert_eq!(actual, expected);
    }

    /// The `fixtures::` builders drive operations through a
    /// `&mut TestStore` (spec #510: the signatures change from
    /// `&mut MemStore` to `&mut TestStore`).
    #[tokio::test]
    async fn fixture_bike_creates_a_bike_in_the_store() -> TbResult<()> {
        let mut store = TestStore::new();
        let s = test_session();

        let bike = fixtures::fixture_bike(&s, &mut store).await?;

        let got = store.partid_get_part(bike.id).await?;
        assert_eq!(got.name, "Main Bike");
        Ok(())
    }

    /// A [`StravaUser`] for the tests (the strava crate's own test
    /// support is private, so the helper lives here).
    fn test_user() -> StravaUser {
        StravaUser {
            id: StravaId::from(42),
            tendabike_id: UserId::from(1),
            refresh_token: None,
        }
    }
}
