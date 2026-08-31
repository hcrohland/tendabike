# Phase 1: Test Infrastructure — MemStore (Detailed Plan)

## Overview

This plan details the implementation of the in-memory mock store (`MemStore`) that implements all 8 store subtraits required for domain and attachment tests. The `Store` trait is modular — it composes 8 independent subtraits plus the `commit()` method.

**Key design decisions:**

1. **Flat-file subtrait modules**: Each subtrait implementation lives in its own flat file (`mem_usage.rs`, `mem_user.rs`, etc.) inside the `test_support/` directory — NOT one monolithic file.
2. **Two-phase implementation**: First, all files are created with `todo!()` stubs and the project must compile cleanly. Only then are method bodies implemented on-demand when tests require them.

```
Store (marker trait)
├── PartStore        → 10 methods for part CRUD and shop registration
├── ActivityStore    → 11 methods for activity CRUD and queries
├── AttachmentStore  → 11 methods for attachment timeline operations
├── UsageStore       → 5 methods for usage accounting
├── ServiceStore     → 6 methods for service timeline operations
├── ServicePlanStore → 7 methods for service plan management
├── UserStore        → 5 methods for user CRUD
└── ShopStore        → 15 methods for shop and subscription management

Session (separate trait)
├── user_id()    -> UserId
├── shop()       -> Option<ShopId>
├── set_shop()   -> TbResult<()>
└── is_admin()   -> bool
```

## File Structure

> **Rule**: No `mod.rs` files. Use flat file modules — declare `mod test_support;` in lib.rs and create a directory `test_support/` with flat `.rs` files.

```
backend/domain/src/test_support.rs              # Root: struct MemStore, TestSession + `mod` declarations
backend/domain/src/test_support/
├── mem_usage.rs                # UsageStore impl for MemStore
├── mem_user.rs                 # UserStore impl for MemStore
├── mem_shop.rs                 # ShopStore impl for MemStore
├── mem_part.rs                 # PartStore impl for MemStore
├── mem_activity.rs             # ActivityStore impl for MemStore
├── mem_attachment.rs           # AttachmentStore impl for MemStore
├── mem_service.rs              # ServiceStore impl for MemStore
└── mem_serviceplan.rs          # ServicePlanStore impl for MemStore
```

The `test_support.rs` root file declares the submodules and re-exports public items:

```rust
// test_support.rs

// NOTE: No #[cfg(test)] needed on individual items here — test_support.rs itself
// is already gated by #[cfg(test)] in lib.rs. All content is implicitly test-only.

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

// MemStore struct definition, TestSession, Store impl, helpers
```

Tests that use MemStore live in `backend/domain/src/entities/attachment.rs` within a `#[cfg(test)] mod tests { }` block (existing pattern), or in separate test files.

## Two-Phase Implementation Strategy

### Phase 1: Stub All Files — Compile Cleanly

**Goal**: Every file exists, every method body is `todo!()`, and `cargo check` passes with zero errors.

**Why**: This establishes the complete structure before any real logic enters. It lets you verify:

- All method signatures are correct (compiler catches mismatches)
- Module declarations compile
- Type references resolve across files
- No missing imports

**Process**: Create all 9 files (root + 8 subtrait stubs) with every method body being `todo!()`. Run `SQLX_OFFLINE=true cargo check -p tb_domain`. Fix any compilation errors until clean.

### Phase 2: On-Demand Implementation (When Tests Need It)

**Goal**: When a test requires a specific method to work, replace that single `todo!()` with a real implementation.

**Why**: Avoids writing implementations for methods that may never be needed by tests. Each implementation is driven by an actual test failure, ensuring every line of code serves a purpose.

**Process**:

1. Write a test that requires a specific method (e.g., `store.partid_get_part(pid).await`)
2. The test will panic at `todo!()` — examine the error to confirm which method is needed
3. Replace that single `todo!()` in the appropriate `mem_*.rs` file with a working implementation
4. Re-run tests to verify

> **No pre-planned implementations.** The stub files serve as a contract — every method signature is verified by the compiler during Phase 1, and implementations are added only when tests demand them.

---

## Step 1: Module Declaration (`lib.rs`)

### File: [`backend/domain/src/lib.rs`](backend/domain/src/lib.rs)

**Purpose**: Add the test support module so it's available under `#[cfg(test)]`.

**Changes Required**: Add at the end of `lib.rs`:

```rust
#[cfg(test)]
pub mod test_support;
```

No changes needed in `entities.rs` — MemStore is a test utility, not an entity type.

---

## Step 2: Root Module + Struct Definition (`test_support.rs`)

### File: [`backend/domain/src/test_support.rs`](backend/domain/src/test_support.rs)

**Purpose**: Define the module structure (imports of 8 subtrait files) and the core `MemStore` struct + `TestSession`.

**Implementation**:

```rust
// backend/domain/src/test_support.rs

// NOTE: No #[cfg(test)] needed on individual items here — test_support.rs itself
// is already gated by #[cfg(test)] in lib.rs. All content is implicitly test-only.

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

use crate::*; // brings in all entities, traits, Error, TbResult

/// Test session for attachment tests
pub struct TestSession {
    user_id: UserId,
    shop: Option<ShopId>,
    admin: bool,
}

impl TestSession {
    pub fn new(user_id: UserId) -> Self {
        Self { user_id, shop: None, admin: false }
    }

    pub fn with_shop(user_id: UserId, shop: ShopId) -> Self {
        Self { user_id, shop: Some(shop), admin: false }
    }

    pub fn with_admin(user_id: UserId, admin: bool) -> Self {
        Self { user_id, shop: None, admin }
    }
}

impl Session for TestSession {
    fn user_id(&self) -> UserId { self.user_id }
    fn shop(&self) -> Option<ShopId> { self.shop }
    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()> {
        self.shop = shop;
        Ok(())
    }
    fn is_admin(&self) -> bool { self.admin }
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

    /// Helper: find next ID for a given type (simple auto-increment)
    fn next_part_id(&self) -> PartId {
        PartId(self.parts.keys().map(|id| id.0).max().unwrap_or(0) + 1)
    }

    fn next_activity_id(&self) -> ActivityId {
        let max_id = self.activities.iter()
            .map(|a| a.id.0)
            .max()
            .unwrap_or(0);
        ActivityId(max_id + 1)
    }

    fn next_shop_id(&self) -> ShopId {
        ShopId(self.shops.keys().map(|id| id.0).max().unwrap_or(0) + 1)
    }

    fn next_user_id(&self) -> UserId {
        let max_id = self.users.keys().map(|id| id.0).max().unwrap_or(0);
        UserId(max_id + 1)
    }

    /// Helper: find next AttachmentId (AttachmentId is a simple i64 newtype)
    fn next_attachment_id(&self) -> AttachmentId {
        let max_id = self.attachments.iter()
            .map(|a| a.id.0)
            .max()
            .unwrap_or(0);
        AttachmentId(max_id + 1)
    }

    /// Helper: find next ServicePlanId (ServicePlanId is a UUID v7)
    /// Use ServicePlanId::new() or generate appropriately.
    fn next_service_plan_id(&self) -> ServicePlanId {
        ServicePlanId::new()
    }
}
```

> **Note**: `ServiceId` uses time-based UUID. Use `ServiceId::new()`. See entity files for available constructors.

---

## Phase 1: Create All Stub Files with `todo!()`

**After Step 2 compiles cleanly, create these 8 stub files. Every method body must be `todo!()`.**

---

## Step 3: UsageStore Stub (`test_support/mem_usage.rs`)

### File: `backend/domain/src/test_support/mem_usage.rs`

**Reference**: [`backend/domain/src/traits/usage.rs`](backend/domain/src/traits/usage.rs)

**Methods** (5): `get`, `update`, `delete`, `usages_delete`, `delete_all`

```rust
use crate::{UsageStore, TbResult, Usage, UsageId};
use std::borrow::Borrow;
use time::OffsetDateTime;

#[async_trait::async_trait]
impl UsageStore for super::MemStore {
    async fn get(&mut self, uid: UsageId) -> TbResult<Option<Usage>> { todo!() }
    async fn update<U>(&mut self, vec: &[U]) -> TbResult<usize> where U: Borrow<Usage> + Sync { todo!() }
    async fn delete(&mut self, uid: UsageId) -> TbResult<Usage> { todo!() }
    async fn usages_delete(&mut self, vec: &[Usage]) -> TbResult<usize> { todo!() }
    async fn delete_all(&mut self) -> TbResult<usize> { todo!() }
}
```

**Verification**: After creating this file, run `SQLX_OFFLINE=true cargo check -p tb_domain`. All method signature mismatches should surface now.

---

## Step 4: UserStore Stub (`test_support/mem_user.rs`)

### File: `backend/domain/src/test_support/mem_user.rs`

**Reference**: [`backend/domain/src/traits/user.rs`](backend/domain/src/traits/user.rs)

**Methods** (5): `get`, `create`, `update`, `user_delete`, `update_onboarding_status`

```rust
use crate::{UserStore, TbResult, User, UserId, OnboardingStatus};

#[async_trait::async_trait]
impl UserStore for super::MemStore {
    async fn get(&mut self, uid: UserId) -> TbResult<User> { todo!() }
    async fn create(&mut self, firstname: &str, lastname: &str, avatar: &Option<String>) -> TbResult<User> { todo!() }
    async fn update(&mut self, uid: &UserId, firstname: &str, lastname: &str, avatar: &Option<String>) -> TbResult<User> { todo!() }
    async fn user_delete(&mut self, user: &UserId) -> TbResult<usize> { todo!() }
    async fn update_onboarding_status(&mut self, uid: &UserId, status: OnboardingStatus) -> TbResult<User> { todo!() }
}
```

---

## Step 5: ShopStore Stub (`test_support/mem_shop.rs`)

### File: `backend/domain/src/test_support/mem_shop.rs`

**Reference**: [`backend/domain/src/traits/shop.rs`](backend/domain/src/traits/shop.rs)

**Methods** (15): `shop_get`, `shop_create`, `shop_update`, `shop_delete`, `shops_get_all_for_user`, `shops_search`, `subscription_create`, `subscription_get`, `subscription_find_active`, `subscription_find_pending`, `subscription_update_status`, `subscription_approve`, `subscription_delete`, `subscriptions_for_shop`, `subscriptions_for_user`

```rust
use crate::{ShopStore, TbResult, Shop, ShopId, ShopSubscription, SubscriptionId, SubscriptionStatus};

#[async_trait::async_trait]
impl ShopStore for super::MemStore {
    async fn shop_get(&mut self, id: ShopId) -> TbResult<Shop> { todo!() }
    async fn shop_create(&mut self, name: String, description: Option<String>, auto_approve: bool, owner: UserId) -> TbResult<Shop> { todo!() }
    async fn shop_update(&mut self, id: ShopId, name: String, description: Option<String>, auto_approve: bool) -> TbResult<Shop> { todo!() }
    async fn shop_delete(&mut self, id: ShopId) -> TbResult<usize> { todo!() }
    async fn shops_get_all_for_user(&mut self, user_id: UserId) -> TbResult<Vec<Shop>> { todo!() }
    async fn shops_search(&mut self, query: &str) -> TbResult<Vec<Shop>> { todo!() }
    async fn subscription_create(&mut self, shop_id: ShopId, user_id: UserId, message: Option<String>) -> TbResult<ShopSubscription> { todo!() }
    async fn subscription_get(&mut self, id: SubscriptionId) -> TbResult<ShopSubscription> { todo!() }
    async fn subscription_find_active(&mut self, shop_id: ShopId, user_id: UserId) -> TbResult<Option<ShopSubscription>> { todo!() }
    async fn subscription_find_pending(&mut self, shop_id: ShopId, user_id: UserId) -> TbResult<Option<ShopSubscription>> { todo!() }
    async fn subscription_update_status(&mut self, id: SubscriptionId, status: SubscriptionStatus) -> TbResult<ShopSubscription> { todo!() }
    async fn subscription_approve(&mut self, id: SubscriptionId, status: SubscriptionStatus, response_message: Option<String>) -> TbResult<ShopSubscription> { todo!() }
    async fn subscription_delete(&mut self, id: SubscriptionId) -> TbResult<()> { todo!() }
    async fn subscriptions_for_shop(&mut self, shop_id: ShopId) -> TbResult<Vec<ShopSubscription>> { todo!() }
    async fn subscriptions_for_user(&mut self, user_id: UserId) -> TbResult<Vec<ShopSubscription>> { todo!() }
}
```

---

## Step 6: PartStore Stub (`test_support/mem_part.rs`)

### File: `backend/domain/src/test_support/mem_part.rs`

**Reference**: [`backend/domain/src/traits/part.rs`](backend/domain/src/traits/part.rs)

**Methods** (10): `partid_get_part`, `part_get_all_for_userid`, `part_create`, `part_update`, `part_delete`, `parts_delete`, `partid_get_by_source`, `parts_register_shop`, `parts_unregister_shop`, `shop_get_parts`

```rust
use crate::{PartStore, TbResult, Part, PartId, PartTypeId, ShopId};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl PartStore for super::MemStore {
    async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> { todo!() }
    async fn part_get_all_for_userid(&mut self, uid: &UserId) -> TbResult<Vec<Part>> { todo!() }
    async fn part_create(&mut self, what: PartTypeId, name: String, vendor: String, model: String, purchase: OffsetDateTime, source: Option<String>, notes: String, usage: UsageId, owner: UserId, shop: Option<ShopId>) -> TbResult<Part> { todo!() }
    async fn part_update(&mut self, part: Part) -> TbResult<Part> { todo!() }
    async fn part_delete(&mut self, part: PartId) -> TbResult<PartId> { todo!() }
    async fn parts_delete(&mut self, parts: &[Part]) -> TbResult<usize> { todo!() }
    async fn partid_get_by_source(&mut self, strava_id: &str) -> TbResult<Option<PartId>> { todo!() }
    async fn parts_register_shop(&mut self, shop_id: ShopId, part_id: Vec<PartId>) -> TbResult<Vec<Part>> { todo!() }
    async fn parts_unregister_shop(&mut self, part_ids: Vec<PartId>) -> TbResult<Vec<Part>> { todo!() }
    async fn shop_get_parts(&mut self, shop_id: ShopId) -> TbResult<Vec<Part>> { todo!() }
}
```

---

## Step 7: ActivityStore Stub (`test_support/mem_activity.rs`)

### File: `backend/domain/src/test_support/mem_activity.rs`

**Reference**: [`backend/domain/src/traits/activity.rs`](backend/domain/src/traits/activity.rs)

**Methods** (11): `activity_create`, `activity_read_by_id`, `activity_update`, `activity_delete`, `activities_delete`, `get_all`, `activities_find_by_gear_and_time`, `get_by_user_and_time`, `activity_set_gear_if_null`, `activity_get_really_all`

> **Note**: Method name is `activity_read_by_id` (not `activity_get_by_id`). Parameter name in `activities_find_by_gear_and_time()` is `part` (not `gear`).

```rust
use crate::{ActivityStore, TbResult, Activity, ActivityId, PartId, PartTypeId as PartType, UserId};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl ActivityStore for super::MemStore {
    async fn activity_create(&mut self, act: Activity) -> TbResult<Activity> { todo!() }
    async fn activity_read_by_id(&mut self, aid: ActivityId) -> TbResult<Option<Activity>> { todo!() }
    async fn activity_update(&mut self, act: Activity) -> TbResult<Activity> { todo!() }
    async fn activity_delete(&mut self, aid: ActivityId) -> TbResult<usize> { todo!() }
    async fn activities_delete(&mut self, activities: &[Activity]) -> TbResult<usize> { todo!() }
    async fn get_all(&mut self, uid: &UserId) -> TbResult<Vec<Activity>> { todo!() }
    async fn activities_find_by_gear_and_time(&mut self, part: PartId, begin: OffsetDateTime, end: OffsetDateTime) -> TbResult<Vec<Activity>> { todo!() }
    async fn get_by_user_and_time(&mut self, uid: UserId, rstart: OffsetDateTime) -> TbResult<Activity> { todo!() }
    async fn activity_set_gear_if_null(&mut self, user: UserId, types: Vec<crate::ActTypeId>, partid: &PartId) -> TbResult<Vec<Activity>> { todo!() }
    async fn activity_get_really_all(&mut self) -> TbResult<Vec<Activity>> { todo!() }
}
```

---

## Step 8: AttachmentStore Stub (`test_support/mem_attachment.rs`)

### File: `backend/domain/src/test_support/mem_attachment.rs`

**Reference**: [`backend/domain/src/traits/attachment.rs`](backend/domain/src/traits/attachment.rs)

**Methods** (11): `attachment_create`, `delete`, `attachments_delete_by_parts`, `attachment_get_by_gear_and_time`, `attachments_all_by_part`, `attachment_get_by_part_and_time`, `assembly_get_by_types_time_and_gear`, `attachment_find_part_of_type_at_hook_and_time`, `attachment_find_successor`, `attachment_find_later_attachment_for_part`, `attachment_find_part_attached_already`

```rust
use crate::{AttachmentStore, TbResult, Attachment, PartId, Part, PartTypeId};
use time::OffsetDateTime;

#[async_trait::async_trait]
impl AttachmentStore for super::MemStore {
    async fn attachment_create(&mut self, att: Attachment) -> TbResult<Attachment> { todo!() }
    async fn delete(&mut self, att: Attachment) -> TbResult<Attachment> { todo!() }
    async fn attachments_delete_by_parts(&mut self, parts: &[Part]) -> TbResult<usize> { todo!() }
    async fn attachment_get_by_gear_and_time(&mut self, gear: PartId, start: OffsetDateTime) -> TbResult<Vec<Attachment>> { todo!() }
    async fn attachments_all_by_part(&mut self, id: PartId) -> TbResult<Vec<Attachment>> { todo!() }
    async fn attachment_get_by_part_and_time(&mut self, pid: PartId, time: OffsetDateTime) -> TbResult<Option<Attachment>> { todo!() }
    async fn assembly_get_by_types_time_and_gear(&mut self, types: Vec<PartTypeId>, gear: PartId, time: OffsetDateTime) -> TbResult<Vec<Attachment>> { todo!() }
    async fn attachment_find_part_of_type_at_hook_and_time(&mut self, what: PartTypeId, gear: PartId, hook: PartTypeId, time: OffsetDateTime) -> TbResult<Option<Attachment>> { todo!() }
    async fn attachment_find_successor(&mut self, part_id: PartId, gear: PartId, hook: PartTypeId, time: OffsetDateTime, what: PartTypeId) -> TbResult<Option<Attachment>> { todo!() }
    async fn attachment_find_later_attachment_for_part(&mut self, part_id: PartId, time: OffsetDateTime) -> TbResult<Option<Attachment>> { todo!() }
    async fn attachment_find_part_attached_already(&mut self, part_id: PartId, gear: PartId, hook: PartTypeId, time: OffsetDateTime) -> TbResult<Option<Attachment>> { todo!() }
}
```

---

## Step 9: ServiceStore Stub (`test_support/mem_service.rs`)

### File: `backend/domain/src/test_support/mem_service.rs`

**Reference**: [`backend/domain/src/traits/service.rs`](backend/domain/src/traits/service.rs)

**Methods** (6): `create`, `get`, `update`, `delete`, `services_delete`, `services_by_part`

```rust
use crate::{ServiceStore, TbResult, Service, ServiceId, PartId};

#[async_trait::async_trait]
impl ServiceStore for super::MemStore {
    async fn create(&mut self, service: Service) -> TbResult<Service> { todo!() }
    async fn get(&mut self, service: ServiceId) -> TbResult<Service> { todo!() }
    async fn update(&mut self, service: Service) -> TbResult<Service> { todo!() }
    async fn delete(&mut self, service: ServiceId) -> TbResult<usize> { todo!() }
    async fn services_delete(&mut self, services: &[Service]) -> TbResult<usize> { todo!() }
    async fn services_by_part(&mut self, part: PartId) -> TbResult<Vec<Service>> { todo!() }
}
```

---

## Step 10: ServicePlanStore Stub (`test_support/mem_serviceplan.rs`)

### File: `backend/domain/src/test_support/mem_serviceplan.rs`

**Reference**: [`backend/domain/src/traits/serviceplan.rs`](backend/domain/src/traits/serviceplan.rs)

**Methods** (7): `create`, `get`, `plan_update`, `delete`, `serviceplans_delete`, `by_part`, `by_user`

> **Note**: Method names are `by_part` and `by_user` (not `for_part`/`for_user`).

```rust
use crate::{ServicePlanStore, TbResult, ServicePlan, ServicePlanId, PartId, UserId};

#[async_trait::async_trait]
impl ServicePlanStore for super::MemStore {
    async fn create(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> { todo!() }
    async fn get(&mut self, plan: ServicePlanId) -> TbResult<ServicePlan> { todo!() }
    async fn plan_update(&mut self, plan: ServicePlan) -> TbResult<ServicePlan> { todo!() }
    async fn delete(&mut self, plan: ServicePlanId) -> TbResult<usize> { todo!() }
    async fn serviceplans_delete(&mut self, serviceplans: &[ServicePlan]) -> TbResult<usize> { todo!() }
    async fn by_part(&mut self, part: PartId) -> TbResult<Vec<ServicePlan>> { todo!() }
    async fn by_user(&mut self, uid: UserId) -> TbResult<Vec<ServicePlan>> { todo!() }
}
```

---

## STEP 1 VERIFICATION: Clean Compilation

After creating all stub files (Steps 3-10), verify:

```bash
SQLX_OFFLINE=true cargo check -p tb_domain
```

This should compile with only `todo!()` panic markers, no type/signature errors. Fix any issues before proceeding to Phase 2.

---

## Phase 2: On-Demand Implementation Workflow

**Implement real method bodies only when tests need them. Each implementation is driven by a failing test.**

### How to Implement On-Demand

1. **Write a test** that exercises a specific method:

   ```rust
   #[tokio::test]
   async fn test_partid_get_part() {
       let mut store = MemStore::new();
       let part = Part { /* ... */ };
       // store.partid_get_part(part.id).await should return Ok(part)
   }
   ```

2. **Run the test** — it will panic at `todo!()` in `mem_part.rs`

3. **Replace the single `todo!()`** with a real implementation:

   ```rust
   async fn partid_get_part(&mut self, pid: PartId) -> TbResult<Part> {
       self.parts.get(&pid).copied()
           .ok_or_else(|| Error::NotFound(format!("Part {} not found", pid)))
   }
   ```

4. **Re-run the test** to confirm it passes

5. **Repeat** for next method as needed

### Implementation Reference Guide

When implementing, refer back to the existing SQL-based implementations in `backend/domain/src/traits/` for method semantics. The HashMap patterns are straightforward:

| Trait            | Collection                                        | Key Pattern                                       |
| ---------------- | ------------------------------------------------- | ------------------------------------------------- |
| UsageStore       | `HashMap<UsageId, Usage>`                         | Direct lookup by ID                               |
| UserStore        | `HashMap<UserId, User>`                           | Direct lookup, filter by owner                    |
| ShopStore        | `HashMap<ShopId, Shop>` + `Vec<ShopSubscription>` | Lookup + filter                                   |
| PartStore        | `HashMap<PartId, Part>`                           | Lookup + filter by owner/shop                     |
| ActivityStore    | `Vec<Activity>`                                   | Linear scan, filter by user/part/time             |
| AttachmentStore  | `Vec<Attachment>`                                 | Timeline queries: `attached <= t && detached > t` |
| ServiceStore     | `Vec<Service>`                                    | Filter by part_id                                 |
| ServicePlanStore | `Vec<ServicePlan>`                                | Filter by part/user                               |

---

## Step 19: Store Trait Implementation (`test_support.rs`)

**Purpose**: Implement the `Store` trait which requires `commit()`. For in-memory store, commit is a no-op since all operations are already in memory.

**Add to `test_support.rs` (at the end, after all `mod` declarations)**:

```rust
#[async_trait::async_trait]
impl Store for MemStore {
    async fn commit(self) -> TbResult<()> {
        // In-memory store - all changes are immediate
        Ok(())
    }
}
```

Verify: `SQLX_OFFLINE=true cargo check -p tb_domain` — should be fully clean now.

---

## Phase 1 Verification Checklist

After all stub files are created:

1. **Full compilation**: `SQLX_OFFLINE=true cargo check -p tb_domain` — zero errors
2. **Build test binary**: `SQLX_OFFLINE=true cargo test -p tb_domain --no-run` — compiles and links

---

## Implementation Order and Dependencies

```
Phase 1 (Stubs - compile clean):

Step 2   → test_support.rs: struct MemStore, TestSession, helper methods
    ↓
Step 3   → mem_usage.rs:      UsageStore stub (5 x todo!())
Step 4   → mem_user.rs:       UserStore stub (5 x todo!())
Step 5   → mem_shop.rs:       ShopStore stub (15 x todo!())
Step 6   → mem_part.rs:       PartStore stub (10 x todo!())
Step 7   → mem_activity.rs:   ActivityStore stub (10 x todo!())
Step 8   → mem_attachment.rs: AttachmentStore stub (11 x todo!())
Step 9   → mem_service.rs:    ServiceStore stub (6 x todo!())
Step 10  → mem_serviceplan.rs: ServicePlanStore stub (7 x todo!())
    ↓
[VERIFY: cargo check - clean, only todo!() panics]
    ↓

Phase 2 (On-demand implementation as tests require):

Step 19  → test_support.rs:   Store impl (commit = no-op)
    ↓
[TEST DRIVEN IMPLEMENTATION - methods implemented only when tests fail]

Example flow:
  1. Write test → panics at todo!() in mem_part.rs
  2. Implement partid_get_part in mem_part.rs
  3. Test passes → move to next method
```

---

## Cargo.toml Verification

The existing [`backend/domain/Cargo.toml`](backend/domain/Cargo.toml) already has the necessary dev-dependencies:

```toml
[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt"] }
time = { workspace = true, features = ["serde"] }
```

No changes needed — `tokio::test` macro and `time` serialization are already available.

---

## Risks and Mitigations

| Risk                                                                     | Mitigation                                                                       |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| AttachmentStore timeline logic is complex                                | Implement one query at a time, test each with minimal data                       |
| `attachment_find_part_of_type_at_hook_and_time()` needs PartStore access | May need to check part `what` field separately or simplify test cases            |
| Self-referential trait bounds (`&mut impl Store`)                        | MemStore implements all traits, so it satisfies the bound                        |
| UsageId/ServiceId use UUID v7 (time-based)                               | Use `UsageId::new()` and `ServiceId::new()` in helpers, never hardcode           |
| PartTypeId values are from `objects.rs`                                  | Use actual `PartTypeId` values or integer casts like `PartTypeId::from(1)`       |
| `MAX_TIME` constant import                                               | Import from `crate::MAX_TIME` — re-exported via `pub use entities::*;` in lib.rs |
| Attachment entity struct fields may differ                               | Verify against actual `Attachment` struct in attachment.rs before coding         |

---

## Summary: Flat-File Subtrait Structure

| Category                 | Revised Plan                                                             |
| ------------------------ | ------------------------------------------------------------------------ |
| MemStore location        | `test_support.rs` root + 8 flat files in `test_support/`                 |
| Phase strategy           | **Phase 1**: all stubs compile → **Phase 2**: implement bodies on-demand |
| PartStore method count   | **10**                                                                   |
| ShopStore method count   | **15**                                                                   |
| AttachmentStore methods  | **11**                                                                   |
| ActivityStore methods    | **10**                                                                   |
| ServiceStore methods     | **6**                                                                    |
| ServicePlanStore methods | **7**                                                                    |
| Module structure         | Root + 8 subtrait files (no `mod.rs` anywhere)                           |
