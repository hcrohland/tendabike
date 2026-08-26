// backend/domain/src/test_support.rs

mod mem_usage;

mod mem_user;

mod mem_shop;

mod mem_part;

mod mem_activity;

mod mem_attachment;

mod mem_service;

mod mem_serviceplan;

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

    /// Attachments for timeline queries
    attachments: HashMap<(PartId, time::OffsetDateTime), Attachment>,

    /// Usages keyed by UsageId
    usages: HashMap<UsageId, Usage>,

    /// Services stored as Vec (need iteration for filter ops)
    services: HashMap<ServiceId, Service>,

    /// Service plans stored as Vec (need iteration for filter ops)
    service_plans: HashMap<ServicePlanId, ServicePlan>,

    /// Users keyed by UserId
    users: HashMap<UserId, User>,

    /// Shops keyed by ShopId
    shops: HashMap<ShopId, Shop>,

    /// Subscriptions
    subscriptions: Vec<ShopSubscription>,

    /// Auto-increment counter for PartId
    next_part_id: i32,

    /// Auto-increment counter for ActivityId
    #[allow(dead_code)]
    next_activity_id: i64,
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
            activities: Vec::new(),
            attachments: HashMap::new(),
            usages: HashMap::new(),
            services: HashMap::new(),
            service_plans: HashMap::new(),
            users: HashMap::new(),
            shops: HashMap::new(),
            subscriptions: Vec::new(),
            next_part_id: 1,
            next_activity_id: 1,
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
