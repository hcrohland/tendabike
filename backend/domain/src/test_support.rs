// backend/domain/src/test_support.rs

mod mem_usage;

mod mem_user;

mod mem_shop;

mod mem_part;

mod mem_activity;

mod mem_attachment;

mod mem_service;

mod mem_serviceplan;

pub mod fixtures;

mod mem_store;
pub use mem_store::{StoreSnapshot, build_workshop_store};

mod prepopulated_data;

// --- Test constants for use in unit tests ---

/// PartTypeId constants for tests (UPPERCASE per Rust naming conventions).
/// These replace the constants that were previously defined in PartTypeId impl block.
pub mod part_type_ids {
    use crate::PartTypeId;

    pub const CHAIN: PartTypeId = PartTypeId::from_id(4);
    pub const BIKE: PartTypeId = PartTypeId::from_id(1);
    pub const REAR_WHEEL: PartTypeId = PartTypeId::from_id(5);
    pub const CASSETTE: PartTypeId = PartTypeId::from_id(9);
    pub const SEATPOST: PartTypeId = PartTypeId::from_id(10);
    pub const SADDLE: PartTypeId = PartTypeId::from_id(11);
    pub const DERAILLEUR: PartTypeId = PartTypeId::from_id(12);
    pub const CRANK: PartTypeId = PartTypeId::from_id(13);
    pub const CHAINRING: PartTypeId = PartTypeId::from_id(14);
    pub const BRAKE_ROTOR: PartTypeId = PartTypeId::from_id(15);
    pub const FORK: PartTypeId = PartTypeId::from_id(16);
    pub const REAR_SHOCK: PartTypeId = PartTypeId::from_id(17);
    pub const HANDLEBAR: PartTypeId = PartTypeId::from_id(18);
    pub const BOTTOM_BRACKET: PartTypeId = PartTypeId::from_id(19);
    pub const HEADSET: PartTypeId = PartTypeId::from_id(20);
    pub const FRONT_WHEEL: PartTypeId = PartTypeId::from_id(2);
    pub const TIRE: PartTypeId = PartTypeId::from_id(3);
    pub const BRAKE_PAD: PartTypeId = PartTypeId::from_id(6);
    pub const FRONT_BRAKE: PartTypeId = PartTypeId::from_id(7);
    pub const REAR_BRAKE: PartTypeId = PartTypeId::from_id(8);
}

// --- Summary assertions ---

/// An [`IdKeyedMap`] holding exactly one entry: the tombstone for `id`.
/// For whole-map assertions: `assert_eq!(summary.parts, only_tombstone(id))`.
pub fn only_tombstone<K, E>(id: K) -> IdKeyedMap<K, E>
where
    K: Eq + Hash,
{
    let mut map = IdKeyedMap::default();
    map.tombstone(id);
    map
}

/// The sole live entity in `map`, cloned out; panics if `map` holds
/// anything other than exactly one live entry. For tests asserting an op
/// reported exactly one entity: `let svc = sole_live(&summary.services)`.
pub fn sole_live<K, E>(map: &IdKeyedMap<K, E>) -> E
where
    E: Clone,
{
    let mut live = map.live();
    assert_eq!(
        live.len(),
        1,
        "expected exactly one live entry, found {}",
        live.len()
    );
    live.pop().expect("one live entry, asserted above")
}

// --- Core types shared by all subtrait impls ---

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::*;

/// A [`MockSession`] for `user`, `shop`, and `admin`, whose `check_owner`
/// behaves like the trait default (delegating to `user_id`). The
/// expectations accept any call count (including zero), so a session built
/// for one op need not predict which of its methods the op calls.
fn build_session(user: UserId, shop: Option<ShopId>, admin: bool) -> MockSession {
    let mut mock = MockSession::new();
    mock.expect_user_id().times(0..).return_const(user);
    mock.expect_shop().times(0..).return_const(shop);
    mock.expect_is_admin().times(0..).return_const(admin);
    mock.expect_check_owner()
        .times(0..)
        .returning(move |owner, error| user.check_owner(owner, error));
    mock
}

/// A customer [`MockSession`] for `user` (no shop, not admin).
pub fn mock_session(user: UserId) -> MockSession {
    build_session(user, None, false)
}

/// A shop-owner [`MockSession`] for `user` acting in `shop` (not admin).
pub fn mock_session_with_shop(user: UserId, shop: ShopId) -> MockSession {
    build_session(user, Some(shop), false)
}

/// An admin [`MockSession`] for `user` (no shop).
pub fn mock_session_admin(user: UserId) -> MockSession {
    build_session(user, None, true)
}

/// The complete state of the in-memory database: every table plus the
/// auto-increment counters.
#[derive(Clone)]
pub struct StoreData {
    /// Parts keyed by PartId
    pub parts: HashMap<PartId, Part>,

    /// Activities stored as Vec (need iteration for time-range queries)
    pub activities: Vec<Activity>,

    /// Attachments, keyed by the database's primary key: (part_id,
    /// attached_time). One row per part per attach instant.
    pub attachments: HashMap<(PartId, time::OffsetDateTime), Attachment>,

    /// Usages keyed by UsageId
    pub usages: HashMap<UsageId, Usage>,

    /// Services stored as Vec (need iteration for filter ops)
    pub services: HashMap<ServiceId, Service>,

    /// Service plans stored as Vec (need iteration for filter ops)
    pub service_plans: HashMap<ServicePlanId, ServicePlan>,

    /// Part notes (metadata) keyed by PartNoteId
    pub part_notes: HashMap<PartNoteId, PartNote>,

    /// File bytes for notes with a file attachment, keyed by PartNoteId
    pub note_files: HashMap<PartNoteId, Vec<u8>>,

    /// Auto-increment counter for PartId
    pub next_part_id: i32,

    /// Auto-increment counter for PartNoteId
    pub next_note_id: i32,

    /// Users keyed by UserId
    pub users: HashMap<UserId, User>,

    /// Shops keyed by ShopId
    pub shops: HashMap<ShopId, Shop>,

    /// Subscriptions keyed by SubscriptionId
    pub subscriptions: HashMap<SubscriptionId, ShopSubscription>,

    /// Auto-increment counters
    pub next_user_id: i32,
    pub next_shop_id: i32,
    pub next_subscription_id: i32,
}

impl StoreData {
    /// An empty database with all id counters starting at 1.
    pub fn empty() -> Self {
        Self {
            parts: HashMap::new(),
            activities: Vec::new(),
            attachments: HashMap::new(),
            usages: HashMap::new(),
            services: HashMap::new(),
            service_plans: HashMap::new(),
            part_notes: HashMap::new(),
            note_files: HashMap::new(),
            next_part_id: 1,
            next_note_id: 1,
            users: HashMap::new(),
            shops: HashMap::new(),
            subscriptions: HashMap::new(),
            next_user_id: 1,
            next_shop_id: 1,
            next_subscription_id: 1,
        }
    }
}

/// In-memory store implementing all nine sub-traits + Store.
///
/// A `MemStore` is a handle on a shared in-memory database: every write
/// lands directly in the shared state, and every read sees it.
/// [`MemStore::begin`] returns a sibling handle on the same state.
/// [`commit`](MemStore::commit) and [`MemStore::rollback`] are no-ops
/// because the in-memory store does not simulate transactions.
pub struct MemStore {
    /// The shared in-memory database state, held by every handle.
    state: Arc<Mutex<StoreData>>,
}

impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemStore {
    /// An empty shared in-memory database.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(StoreData::empty())),
        }
    }

    /// A guard on the shared state; panics if the lock is poisoned.
    fn state(&self) -> MutexGuard<'_, StoreData> {
        self.state.lock().expect("in-memory store state poisoned")
    }

    /// A cheap sibling handle on the SAME shared state: writes through
    /// either handle are immediately visible to the other.
    pub fn begin(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }

    /// No-op: writes already landed in the shared state. Consumes the
    /// handle, like `SqlxConn::commit`.
    pub async fn commit(self) -> TbResult<()> {
        Ok(())
    }

    /// No-op: writes already landed in the shared state.
    pub async fn rollback(&mut self) -> TbResult<()> {
        Ok(())
    }
}

impl Store for MemStore {}
