//! The domain half of the in-memory database state (moved from
//! `tb_domain::test_support`; the domain copy is deleted in the final
//! migration step, spec #510).

use std::collections::HashMap;

use tb_domain::{
    Activity, Attachment, Part, PartId, PartNote, PartNoteId, Service, ServiceId, ServicePlan,
    ServicePlanId, Shop, ShopId, ShopSubscription, SubscriptionId, Usage, UsageId, User, UserId,
};

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
