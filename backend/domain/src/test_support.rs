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

use std::collections::HashMap;

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

/// In-memory store implementing all 8 subtraits + Store
pub struct MemStore {
    /// Parts keyed by PartId
    parts: HashMap<PartId, Part>,

    /// Activities stored as Vec (need iteration for time-range queries)
    activities: Vec<Activity>,

    /// Attachments for timeline queries (keyed by part_id, attached_time, and unique counter)
    attachments: HashMap<(PartId, time::OffsetDateTime, u64), Attachment>,
    attachment_counter: u64,

    /// Usages keyed by UsageId
    usages: HashMap<UsageId, Usage>,

    /// Services stored as Vec (need iteration for filter ops)
    services: HashMap<ServiceId, Service>,

    /// Service plans stored as Vec (need iteration for filter ops)
    service_plans: HashMap<ServicePlanId, ServicePlan>,

    /// Auto-increment counter for PartId
    next_part_id: i32,
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
            activities: Vec::new(),
            attachments: HashMap::new(),
            attachment_counter: 0,
            usages: HashMap::new(),
            services: HashMap::new(),
            service_plans: HashMap::new(),
            next_part_id: 1,
        }
    }
}

#[async_trait::async_trait]
impl Store for MemStore {
    async fn commit(self) -> TbResult<()> {
        // In-memory store - all changes are immediate
        Ok(())
    }
}
