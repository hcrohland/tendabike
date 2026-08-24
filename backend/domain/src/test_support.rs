// backend/domain/src/test_support.rs

mod mem_usage;
pub use mem_usage::*;

mod mem_user;
pub use mem_user::*;

mod mem_shop;
pub use mem_shop::*;

mod mem_part;
pub use mem_part::*;

mod mem_activity;
pub use mem_activity::*;

mod mem_attachment;
pub use mem_attachment::*;

mod mem_service;
pub use mem_service::*;

mod mem_serviceplan;
pub use mem_serviceplan::*;

// --- Core types shared by all subtrait impls ---

use std::borrow::Borrow;
use std::collections::HashMap;
use time::OffsetDateTime;

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
}

/// In-memory store implementing all 8 subtraits + Store
pub struct MemStore {
    /// Parts keyed by PartId
    parts: HashMap<PartId, Part>,

    /// Activities stored as Vec (need iteration for time-range queries)
    activities: Vec<Activity>,

    /// Attachments for timeline queries
    attachments: Vec<Attachment>,

    /// Usages keyed by UsageId
    usages: HashMap<UsageId, Usage>,

    /// Services stored as Vec (need iteration for filter ops)
    services: Vec<Service>,

    /// Service plans stored as Vec (need iteration for filter ops)
    service_plans: Vec<ServicePlan>,

    /// Users keyed by UserId
    users: HashMap<UserId, User>,

    /// Shops keyed by ShopId
    shops: HashMap<ShopId, Shop>,

    /// Subscriptions
    subscriptions: Vec<ShopSubscription>,
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            parts: HashMap::new(),
            activities: Vec::new(),
            attachments: Vec::new(),
            usages: HashMap::new(),
            services: Vec::new(),
            service_plans: Vec::new(),
            users: HashMap::new(),
            shops: HashMap::new(),
            subscriptions: Vec::new(),
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
