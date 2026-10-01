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

// --- Core types shared by all subtrait impls ---

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::*;

/// Test session for attachment tests
pub struct TestSession {
    user_id: UserId,
    shop: Option<ShopId>,
    admin: bool,
}

impl TestSession {
    pub fn new(user_id: UserId) -> Self {
        Self {
            user_id,
            shop: None,
            admin: false,
        }
    }

    pub fn with_shop(user_id: UserId, shop: ShopId) -> Self {
        Self {
            user_id,
            shop: Some(shop),
            admin: false,
        }
    }

    pub fn with_admin(user_id: UserId, admin: bool) -> Self {
        Self {
            user_id,
            shop: None,
            admin,
        }
    }
}

impl Session for TestSession {
    fn user_id(&self) -> UserId {
        self.user_id
    }
    fn shop(&self) -> Option<ShopId> {
        self.shop
    }
    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()> {
        self.shop = shop;
        Ok(())
    }
    fn is_admin(&self) -> bool {
        self.admin
    }

    // Unused parameters in check_owner use the default implementation
    fn check_owner(&self, owner: UserId, error: String) -> crate::TbResult<()> {
        self.user_id.check_owner(owner, error)
    }
}

/// The complete state of the in-memory database: every table plus the
/// auto-increment counters.
///
/// A [`MemStore`] is a transaction on top of one of these, mirroring the
/// production `SqlxConn` (an open Postgres transaction on the database).
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

/// Test-only fault injection for the in-memory store (issue #409):
/// makes one specific store method fail, so a test can drive a domain
/// operation to fail mid-transaction — something the production store
/// cannot be made to do. Arm it with [`MemStore::fail_next`]; the next
/// call of the armed kind fails with [`Error::DatabaseFailure`], like a
/// database failure would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The next `AttachmentStore::attachment_create` call fails.
    AttachmentCreate,
    /// The next `UsageStore::update` call fails.
    UsageUpdate,
}

/// In-memory store implementing all 8 subtraits + Store.
///
/// A `MemStore` is a transaction on an in-memory database, mirroring the
/// production `SqlxConn` (an open Postgres transaction): every write lands
/// in this transaction's working copy only, and reads see the working copy
/// when it exists, otherwise the committed state.
/// [`commit`](Store::commit) merges the working copy into the database;
/// dropping the store without commit, or calling [`MemStore::rollback`],
/// discards it. [`MemStore::begin`] opens a sibling transaction on the same
/// database, so commit/abort are observable (issue #409).
pub struct MemStore {
    /// The committed database state, shared with sibling transactions.
    /// Production analogue: the database behind the open transaction.
    base: Arc<RwLock<StoreData>>,
    /// This transaction's uncommitted writes (a full working copy, taken
    /// lazily on the first write). `None` means "nothing pending": reads
    /// go straight to the committed state, so a read-only sibling always
    /// sees other transactions' commits. Once it exists, the working copy
    /// is a snapshot: this transaction's writes are visible, commits of
    /// other transactions are not (re-`begin()` to pick them up) — close
    /// enough to the database for test purposes (issue #409).
    buffer: Option<StoreData>,
    /// Armed test-only fault (see [`Fault`]).
    fault: Option<Fault>,
}

impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemStore {
    /// An empty in-memory database with one fresh transaction on it.
    pub fn new() -> Self {
        Self {
            base: Arc::new(RwLock::new(StoreData::empty())),
            buffer: None,
            fault: None,
        }
    }

    /// Begin a sibling transaction on the same in-memory database.
    ///
    /// Siblings see only committed state: uncommitted writes of the other
    /// transaction are invisible to it, become visible once the other
    /// commits, and vanish when the other is dropped or rolled back
    /// (issue #409).
    pub fn begin(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
            buffer: None,
            fault: None,
        }
    }

    /// Abort this transaction: discard every uncommitted write, restoring
    /// the committed state (the in-memory mirror of the database rolling
    /// back an uncommitted transaction; issue #409).
    pub async fn rollback(&mut self) -> TbResult<()> {
        self.buffer = None;
        self.fault = None;
        Ok(())
    }

    /// Arm a test-only fault: the next call of the given kind fails with
    /// [`Error::DatabaseFailure`]. See [`Fault`].
    pub fn fail_next(&mut self, fault: Fault) {
        self.fault = Some(fault);
    }

    /// Fail the armed fault, if the next call of kind `f` is armed.
    fn check_fault(&mut self, f: Fault) -> TbResult<()> {
        if self.fault == Some(f) {
            self.fault = None;
            return Err(Error::DatabaseFailure(anyhow::anyhow!(
                "injected fault: {:?}",
                f
            )));
        }
        Ok(())
    }

    /// The state this transaction sees: the uncommitted working copy when
    /// there is one, otherwise the committed state (cloned). Read methods
    /// use this; it never materializes the buffer.
    pub fn state(&self) -> Cow<'_, StoreData> {
        match &self.buffer {
            Some(buffer) => Cow::Borrowed(buffer),
            None => Cow::Owned(
                self.base
                    .read()
                    .expect("in-memory store base poisoned")
                    .clone(),
            ),
        }
    }

    /// Mutable access to the working copy, materializing it from the
    /// committed state on the first write. Write methods use this.
    pub fn state_mut(&mut self) -> &mut StoreData {
        if self.buffer.is_none() {
            self.buffer = Some(
                self.base
                    .read()
                    .expect("in-memory store base poisoned")
                    .clone(),
            );
        }
        self.buffer
            .as_mut()
            .expect("buffer materialized by state_mut")
    }
}

#[async_trait::async_trait]
impl Store for MemStore {
    async fn commit(self) -> TbResult<()> {
        // Merge this transaction's working copy into the database (the
        // in-memory mirror of `COMMIT`): uncommitted writes land here,
        // where sibling transactions can see them (issue #409).
        let Self {
            base,
            buffer,
            fault: _,
        } = self;
        let mut committed = base.write().expect("in-memory store base poisoned");
        if let Some(buffer) = buffer {
            *committed = buffer;
        }
        Ok(())
    }
}
