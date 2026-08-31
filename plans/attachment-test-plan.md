# Attachment Entity Test Plan

## Overview

This plan defines a comprehensive test strategy for the [`Attachment`](backend/domain/src/entities/attachment.rs) entity and its associated functions. The attachment module is the most complex component in the TendaBike domain layer — it manages hierarchical part-to-gear relationships with timeline-based semantics and drives usage accounting across all attached parts.

## Scope

### In Scope

- All 4 public API functions in `attachment.rs`
- All 5 free functions (helpers) in `attachment.rs`
- All methods on the `Attachment` struct
- The `SumHash` accumulation mechanism used across assembly operations
- Timeline-based attachment semantics

### Out of Scope (Separate Plans)

- `AttachmentStore` trait implementations (PostgreSQL layer) — tested via integration tests
- `Service` entity — tested separately
- `Activity` entity — tested separately
- Frontend attachment components — tested separately

---

## Architecture

The attachment module has a layered dependency structure:

```
┌─────────────────────────────────────────────────────────────┐
│  Public API (4 functions)                                   │
│  ┌──────────────────┐  ┌──────────────────┐                 │
│  │ attach_assembly() │  │detach_assembly() │                │
│  └────────┬─────────┘  └────────┬─────────┘                 │
│  ┌──────────────────┐  ┌────────┴──────────┐                │
│  │dispose_assembly()│  │recover_assembly() │                │
│  └────────┬─────────┘  └───────────────────┘                │
└───────────┼─────────────────────────────────────────────────┘
            │ calls
┌───────────▼───────────────────────────────────────────────────┐
│  Free Functions (5)                                           │
│  ┌────────────┐ ┌────────────┐ ┌─────────────────┐           │
│  │attach_one() │ │shift_subparts│ │dispose_subparts│          │
│  └─────┬──────┘ └─────┬───────┘ └────────┬────────┘           │
│  ┌─────▼──────┐ ┌─────▼──────────────────┴──────┐             │
│  │subparts()  │ │  subattachments()             │             │
│  └────────────┘ └───────────────────────────────┘             │
└───────────────────────────┬───────────────────────────────────┘
                            │ calls
┌───────────────────────────▼───────────────────────────────────┐
│  Attachment Methods (11)                                      │
│  ┌──────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │new()         │  │create()     │  │delete()             │  │
│  │calculate_    │  │detach()     │  │shift()              │  │
│  │usage()       │  │detach_      │  │read_details()       │  │
│  │             │  │assembly()   │  │add_details()        │  │
│  │usage()       │  └─────────────┘  └─────────────────────┘  │
│  └──────────────┘                                              │
│  ┌──────────────────────────────────────────────────────┐     │
│  │  Static (pub(crate)):                                │     │
│  │  activities_by_part()                                 │     │
│  │  for_part_with_usage()                                │     │
│  │  register_activity()                                  │     │
│  └──────────────────────────────────────────────────────┘     │
└───────────────────────────┬───────────────────────────────────┘
                            │ depends on
┌───────────────────────────▼───────────────────────────────────┐
│  Store Trait (8 composed traits)                              │
│  ┌─────────────┐ ┌─────────────┐ ┌────────────────────────┐  │
│  │PartStore    │ │ActivityStore│ │UsageStore              │  │
│  └─────────────┘ └─────────────┘ └────────────────────────┘  │
│  ┌─────────────┐ ┌─────────────┐ ┌────────────────────────┐  │
│  │UserStore    │ │ShopStore    │ │ServiceStore            │  │
│  └─────────────┘ └─────────────┘ └────────────────────────┘  │
│  ┌─────────────────────────────────────┐                     │
│  │ServicePlanStore                     │                     │
│  └─────────────────────────────────────┘                     │
│                                                               │
│  Total: 12+ async methods across all traits                   │
└───────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Test Infrastructure — MemStore

### 1.1 MemStore Design

Create an in-memory mock store that implements all required traits. This is the foundation for all unit tests.

```
backend/domain/src/entities/attachment/mem_store.rs  (NEW FILE)
```

The `MemStore` must implement:

| Trait              | Methods Required                                                                                                                                                                                                                                                                                                                                                         | Notes                                          |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------- |
| `PartStore`        | `partid_get_part`, `part_update`, `part_create`, `part_delete`, `parts_delete`, `part_get_all_for_userid`, `partid_get_by_source`, `parts_register_shop`, `parts_unregister_shop`, `shop_get_parts`                                                                                                                                                                      | Use `HashMap<PartId, Part>`                    |
| `AttachmentStore`  | `attachment_create`, `delete`, `attachments_delete_by_parts`, `attachment_get_by_gear_and_time`, `attachments_all_by_part`, `attachment_get_by_part_and_time`, `assembly_get_by_types_time_and_gear`, `attachment_find_part_of_type_at_hook_and_time`, `attachment_find_successor`, `attachment_find_later_attachment_for_part`, `attachment_find_part_attached_already` | Use `Vec<Attachment>` with timestamp filtering |
| `UsageStore`       | `get`, `update`, `delete`, `delete_all`                                                                                                                                                                                                                                                                                                                                  | Use `HashMap<UsageId, Usage>`                  |
| `ActivityStore`    | `activity_get_by_id`, `activity_create`, `activity_update`, `activity_delete`, `activity_delete_by_parts`, `activity_get_all_for_userid`, `activity_find`, `activity_get_strava_id`, `activity_get_categories`                                                                                                                                                           | Use `Vec<Activity>` with time-range queries    |
| `ServiceStore`     | `service_create`, `service_update`, `service_delete`, `service_get_by_id`, `service_get_all_for_userid`, `service_delete_by_parts`, `service_get_categories`                                                                                                                                                                                                             | Minimal: only what attachment code calls       |
| `ServicePlanStore` | `for_part`, `for_user`                                                                                                                                                                                                                                                                                                                                                   | Minimal stubs                                  |
| `UserStore`        | `user_get_by_id`, `user_public_by_id`, `user_update`                                                                                                                                                                                                                                                                                                                     | Stub: return dummy User                        |
| `ShopStore`        | `shop_create`, `shop_update`, `shop_delete`, `shop_get_by_id`, `shop_get_all_for_user`, `shop_get_users`, `shop_register_part`, `shop_unregister_part`, `shop_get_parts`, `shop_get_part`                                                                                                                                                                                | Stub: no shops in attachment tests             |
| `Store`            | `commit()`                                                                                                                                                                                                                                                                                                                                                               | Always succeeds                                |

### 1.2 Helper Functions

```rust
// In mem_store.rs or test helper module

/// Create a test Part with given id, what (PartTypeId), owner, and optional shop
fn test_part(id: i32, what: PartTypeId, owner: UserId, shop: Option<ShopId>) -> Part

/// Create a test Session with given user_id, shop, and admin flag
fn test_session(user_id: UserId, shop: Option<ShopId>, admin: bool) -> TestSession

/// Create a test Activity with given usage values
fn test_activity(id: i32, gear: PartId, usage: Usage) -> Activity

/// Create a MemStore pre-populated with parts and attachments
fn test_store(parts: Vec<Part>, attachments: Vec<Attachment>, activities: Vec<Activity>) -> MemStore

/// Constants for common PartTypeIds (use actual values from objects.rs)
const FRAME: PartTypeId = ...;
const WHEEL: PartTypeId = ...;
const TIRE: PartTypeId = ...;
const CHAIN: PartTypeId = ...;
const CASSETTE: PartTypeId = ...;
```

### 1.3 Cargo.toml Configuration

Enable testing in the domain crate:

```toml
# backend/domain/Cargo.toml
[dev-dependencies]
tokio = { version = "1", features = ["full"] }
time = { version = "0.3", features = ["serde"] }
```

---

## Phase 2: Private Method Tests

These tests verify the internal logic of `Attachment` struct methods in isolation.

### 2.1 `Attachment::new()` Tests

| Test Name                            | Input                                    | Expected                                  |
| ------------------------------------ | ---------------------------------------- | ----------------------------------------- |
| `test_new_creates_with_valid_params` | Valid PartId, time, gear, hook, detached | Creates Attachment with UsageId::now_v7() |
| `test_new_generates_unique_usage_id` | Same inputs twice                        | Different UsageId values                  |

### 2.2 `Attachment::calculate_usage()` Tests

| Test Name                                  | Setup                                             | Expected                                                 |
| ------------------------------------------ | ------------------------------------------------- | -------------------------------------------------------- |
| `test_calculate_usage_no_activities`       | Attachment with no activities in time range       | Returns Usage::new(usage_id) with all zeros              |
| `test_calculate_usage_single_activity`     | One activity within [attached, detached)          | Returns aggregated usage from that activity              |
| `test_calculate_usage_multiple_activities` | Multiple activities overlapping attachment window | Sums usage from all matching activities                  |
| `test_calculate_usage_partial_overlap`     | Activity partially outside attachment time range  | Only counts activities within range via Activity::find() |
| `test_calculate_usage_still_attached`      | detached == MAX_TIME, activities extend far       | Finds all activities from attached to MAX_TIME           |

### 2.3 `Attachment::add_details()` Tests

| Test Name                                  | Input                | Expected                                       |
| ------------------------------------------ | -------------------- | ---------------------------------------------- |
| `test_add_details_populates_name_and_type` | Part with name, what | AttachmentDetail with name and what fields set |
| `test_add_details_empty_name`              | name=""              | AttachmentDetail preserves empty name          |

### 2.4 `Attachment::usage()` Tests

| Test Name                            | Setup                      | Expected                     |
| ------------------------------------ | -------------------------- | ---------------------------- |
| `test_usage_returns_stored_usage`    | MemStore with usage in map | Returns correct Usage        |
| `test_usage_missing_returns_default` | No usage in map            | Returns Usage::new(usage_id) |

---

## Phase 3: Free Function Tests

These tests cover the module-level helper functions that do not take a `&dyn Session`.

### 3.1 `attach_one()` Tests

This is the core attachment logic — successor detection, predecessor merging, hook conflict resolution.

| Test Name                                  | Setup                                                    | Expected                                  |
| ------------------------------------------ | -------------------------------------------------------- | ----------------------------------------- |
| `test_attach_one_simple`                   | Empty store, new part to gear                            | Creates attachment, returns MAX_TIME      |
| `test_attach_one_with_successor`           | Part A attached at T1, attaching at T0 < T1              | New attachment ends at T1 (next.attached) |
| `test_attach_one_merges_adjacent`          | Attachment [T0, T2), new [T2, MAX)                       | Merges into [T0, MAX) via prev.detach()   |
| `test_attach_one_replaces_later`           | Same part, gear, hook: prev [T0, T4), new [T1, T3)       | Deletes next, new ends at next.detached   |
| `test_attach_one_different_hook_later`     | Part attached to hook B at T2, attaching to hook A at T0 | New attachment ends at T2 (next.attached) |
| `test_attach_one_same_part_different_gear` | Part attached to gear B at T2, attaching to gear A at T0 | Ends at T2                                |
| `test_attach_one_no_existing_attachment`   | No attachments for part                                  | Creates new attachment ending at MAX_TIME |

### 3.2 `shift_subparts()` Tests

| Test Name                            | Setup                                    | Expected                       |
| ------------------------------------ | ---------------------------------------- | ------------------------------ |
| `test_shift_subparts_single_subpart` | Subpart attached to from, shifting to to | Subpart moved to to at time    |
| `test_shift_subparts_no_subparts`    | No subparts attached                     | No-op, returns Ok(())          |
| `test_shift_subparts_nested`         | Two levels of subparts                   | All recursive subparts shifted |

### 3.3 `subattachments()` Tests

| Test Name                              | Setup                              | Expected                                     |
| -------------------------------------- | ---------------------------------- | -------------------------------------------- |
| `test_subattachments_returns_children` | Part with subtypes has attachments | Returns attachments where parent is the part |
| `test_subattachments_empty`            | Part has no subpart attachments    | Empty Vec                                    |

### 3.4 `subparts()` Tests

| Test Name                   | Setup                   | Expected                                   |
| --------------------------- | ----------------------- | ------------------------------------------ |
| `test_subparts_returns_ids` | Three subparts attached | Returns Vec<PartId> with exactly three ids |
| `test_subparts_empty`       | No subparts             | Empty Vec                                  |

---

## Phase 4: Public API Function Tests

These tests exercise the full public API surface. Each takes `&dyn Session` and `&mut impl Store`.

### 4.1 `attach_assembly()` Tests

This is the most critical function — handles full assembly attach with subparts, successor/predecessor chain handling.

#### 4.1.1 Validation Tests

| Test Name                                   | Setup                                     | Expected            |
| ------------------------------------------- | ----------------------------------------- | ------------------- |
| `test_attach_assembly_hook_not_in_parttype` | Part type does not declare this hook      | `Error::BadRequest` |
| `test_attach_assembly_invalid_gear_type`    | Part type not compatible with gear type   | `Error::BadRequest` |
| `test_attach_assembly_user_not_owner`       | User trying to attach another user's part | `Error::Forbidden`  |

#### 4.1.2 Basic Attach Tests

| Test Name                               | Setup                              | Expected                                                     |
| --------------------------------------- | ---------------------------------- | ------------------------------------------------------------ |
| `test_attach_assembly_simple`           | New part to new gear, no conflicts | Creates attachment, returns Summary with part and attachment |
| `test_attach_assembly_with_subparts`    | Part has subparts, all=true        | Subparts also attached to new gear                           |
| `test_attach_assembly_without_subparts` | Part has subparts, all=false       | Subparts left behind                                         |

#### 4.1.3 Successor/Predecessor Tests

| Test Name                          | Setup                                                      | Expected                                     |
| ---------------------------------- | ---------------------------------------------------------- | -------------------------------------------- |
| `test_attach_detaches_predecessor` | Gear already has different part at hook                    | Detaches predecessor assembly first          |
| `test_attach_detaches_self_first`  | Part already attached to different gear                    | Detaches self from old gear, attaches to new |
| `test_attach_replaces_same_part`   | Same part, different time on same gear                     | Merges timeline, updates attachment          |
| `test_attach_successor_chain`      | Part A at hook, Part B at hook, attach Part C at same time | A and B detached, C attached                 |

#### 4.1.4 Assembly Reattachment Tests (all=true)

| Test Name                                     | Setup                                      | Expected                    |
| --------------------------------------------- | ------------------------------------------ | --------------------------- |
| `test_attach_all_reattaches_subparts`         | Part with subparts, reattach with all=true | Subparts reattached to part |
| `test_attach_all_preserves_subpart_hierarchy` | Nested subparts, reattach with all=true    | Full hierarchy preserved    |
| `test_attach_all_no_subparts_to_reattach`     | Part without subparts, all=true            | No additional operations    |

#### 4.1.5 Summary Tests

| Test Name                                      | Setup                             | Expected                                 |
| ---------------------------------------------- | --------------------------------- | ---------------------------------------- |
| `test_attach_returns_summary_with_parts`       | Successful attach                 | Summary contains the part                |
| `test_attach_returns_summary_with_attachments` | Successful attach                 | Summary contains attachment with details |
| `test_attach_returns_summary_with_usages`      | Successful attach with activities | Summary contains updated usages          |

### 4.2 `detach_assembly()` Tests

| Test Name                             | Setup                                     | Expected                                     |
| ------------------------------------- | ----------------------------------------- | -------------------------------------------- |
| `test_detach_assembly_simple`         | Part attached, detach with all=false      | Detaches only the part, not subparts         |
| `test_detach_assembly_with_all`       | Part with subparts attached, all=true     | All subparts also detached (shifted to gear) |
| `test_detach_assembly_not_attached`   | Part not currently attached               | `Error::NotFound("part not attached")`       |
| `test_detach_assembly_user_not_owner` | Non-owner tries to detach                 | `Error::Forbidden`                           |
| `test_detach_adjacent_merges`         | Detach at boundary of adjacent attachment | Adjacent attachments merged correctly        |

### 4.3 `dispose_assembly()` Tests

| Test Name                                      | Setup                                              | Expected                                    |
| ---------------------------------------------- | -------------------------------------------------- | ------------------------------------------- |
| `test_dispose_assembly_simple`                 | Part not attached, dispose                         | Part marked disposed_at, returns summary    |
| `test_dispose_assembly_attached`               | Part currently attached                            | `Error::Conflict("Cannot dispose...")`      |
| `test_dispose_assembly_with_disposed_subparts` | Subparts already disposed, all=true                | Returns summary without disposing again     |
| `test_dispose_subparts_detached_not_disposed`  | Subpart detached at time, all=false                | Detaches subpart instead of disposing       |
| `test_dispose_subparts_disposes_when_attached` | Subpart still attached, all=false                  | Disposes the subpart                        |
| `test_recover_disposed_part`                   | Disposed part, recover with all=false              | Part restored, disposed_at cleared          |
| `test_recover_with_all`                        | Disposed part with subparts, recover with all=true | All subparts restored                       |
| `test_recover_non_disposed`                    | Non-disposed part                                  | `Error::BadRequest("Part is not disposed")` |

### 4.4 `is_attached()` Tests

| Test Name                               | Setup                                    | Expected      |
| --------------------------------------- | ---------------------------------------- | ------------- |
| `test_is_attached_true`                 | Part attached at time                    | Returns true  |
| `test_is_attached_false`                | Part not attached at time                | Returns false |
| `test_is_attached_still_attached`       | Part attached with detached=MAX_TIME     | Returns true  |
| `test_is_attached_detached_before_time` | Part detached at T1, querying at T2 > T1 | Returns false |

### 4.5 `register_activity()` Tests

| Test Name                                  | Setup                        | Expected                                       |
| ------------------------------------------ | ---------------------------- | ---------------------------------------------- |
| `test_register_activity_no_gear`           | gear=None                    | Returns empty Summary                          |
| `test_register_activity_single_part`       | One attachment at gear       | Updates usage for attachment and gear          |
| `test_register_activity_assembly`          | Multiple parts in assembly   | Updates usage for all parts and their services |
| `test_register_activity_updates_last_used` | Activity at time > last_used | Part.last_used updated                         |

### 4.6 `activities_by_part()` Tests

| Test Name                                      | Setup                                 | Expected                              |
| ---------------------------------------------- | ------------------------------------- | ------------------------------------- |
| `test_activities_by_part_overlapping`          | Part attached [T1,T3), activity at T2 | Activity included                     |
| `test_activities_by_part_outside_range`        | Part attached [T1,T2), query [T3,T4)  | No activities                         |
| `test_activities_by_part_multiple_attachments` | Part has two attachment periods       | Activities from both periods returned |

### 4.7 `for_part_with_usage()` Tests

| Test Name                                  | Setup                    | Expected                                    |
| ------------------------------------------ | ------------------------ | ------------------------------------------- |
| `test_for_part_with_usage_returns_details` | Part with attachments    | Returns (Vec<AttachmentDetail>, Vec<Usage>) |
| `test_for_part_with_usage_empty`           | Part with no attachments | Returns empty vectors                       |

---

## Phase 5: Invariant and Property Tests

These tests verify invariants that must hold across all operations.

### 5.1 Timeline Invariants

| Test Name                         | Description                                                                         |
| --------------------------------- | ----------------------------------------------------------------------------------- |
| `test_no_overlapping_attachments` | After any attach/detach, no two attachments for same part overlap on same gear+hook |
| `test_hook_exclusivity`           | At any point in time, only one part attached to a given hook on a gear              |
| `test_timeline_continuity`        | Adjacent attachments for same part on same hook merge correctly                     |
| `test_still_attached_semantics`   | "Still attached" (detached=MAX_TIME) behaves correctly in all lookups               |

### 5.2 Usage Conservation

| Test Name                              | Description                                                |
| -------------------------------------- | ---------------------------------------------------------- |
| `test_attach_creates_usage`            | Attach creates a new usage entry                           |
| `test_detach_removes_usage`            | Detach removes the attachment's usage (via negative usage) |
| `test_attach_update_part_usage`        | Part's usage is updated when attached                      |
| `test_service_recalculation_on_attach` | Services on attached part are recalculated                 |
| `test_service_recalculation_on_detach` | Services are recalculated on detach                        |

### 5.3 SumHash Correctness

| Test Name                    | Description                                           |
| ---------------------------- | ----------------------------------------------------- |
| `test_sumhash_deduplication` | SumHash correctly deduplicates items by key           |
| `test_sumhash_add_assign`    | Adding two summaries produces correct merged result   |
| `test_sumhash_from_summary`  | Converting Summary to SumHash and back preserves data |

### 5.4 Ownership and Shop Rules

| Test Name                     | Description                                                |
| ----------------------------- | ---------------------------------------------------------- |
| `test_attach_same_owner`      | User can attach their own parts to their own gear          |
| `test_attach_same_shop`       | Shop can attach parts within same shop                     |
| `test_attach_cross_ownership` | User without shop access cannot attach another user's part |

---

## Phase 6: Integration Scenarios

End-to-end scenarios combining multiple operations in sequence.

### 6.1 Bike Assembly Lifecycle

```
1. Create FRAME (main part)
2. Create WHEEL, attach to FRAME
3. Create TIRE, attach to WHEEL
4. Create CHAIN, attach to FRAME
5. Register activity on FRAME
6. Verify CHAIN usage increased
7. Detach WHEEL from FRAME
8. Verify TIRE still attached to WHEEL (not to FRAME)
9. Dispose CHAIN
10. Verify CHAIN is disposed but not WHEEL
```

### 6.2 Gear Swap Scenario

```
1. Create FRAME_A, FRAME_B
2. Create WHEEL, attach to FRAME_A
3. Attach WHEEL to FRAME_B (should detach from FRAME_A first)
4. Verify FRAME_A has no WHEEL attachment
5. Verify FRAME_B has WHEEL attachment
```

### 6.3 Part Replacement Scenario

```
1. Create CHAIN_OLD, attach to FRAME
2. Register some activities on FRAME
3. Create CHAIN_NEW
4. Attach CHAIN_NEW to FRAME (replaces CHAIN_OLD)
5. Verify CHAIN_OLD has attachment ending at replacement time
6. Verify CHAIN_NEW has attachment starting at replacement time
```

### 6.4 Subpart Cascade

```
1. Create FRAME, WHEEL, TIRE, BRAKE
2. Attach WHEEL to FRAME
3. Attach TIRE to WHEEL
4. Attach BRAKE to FRAME
5. Detach FRAME with all=true
6. Verify WHEEL is detached from FRAME
7. Verify TIRE is still attached to WHEEL (shifted, not detached)
8. Attach FRAME back to itself (or to another gear)
9. Attach WHEEL back to FRAME with all=true
10. Verify TIRE reattached to WHEEL
```

---

## Test File Structure

Tests and MemStore will be organized into separate files under a dedicated test module directory:

```
backend/domain/src/entities/attachment/
  mod.rs          (module declaration, re-exports for tests)
  mem_store.rs    (MemStore implementation for all 8 store traits)
  helpers.rs      (test helpers: test_part(), test_session(), test_activity(), etc.)
  private_tests.rs      (Phase 2: private method tests)
  free_function_tests.rs  (Phase 3: free function tests)
  public_api_tests.rs     (Phase 4: public API function tests)
  invariant_tests.rs      (Phase 5: invariant and property tests)
  integration_tests.rs    (Phase 6: integration scenario tests)
```

### File Responsibilities

| File                     | Contents                                                                                                                                                                                |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mod.rs`                 | Module declarations, `#[cfg(test)]` gating, shared imports via `use super::*`                                                                                                           |
| `mem_store.rs`           | `MemStore` struct and all trait implementations (`PartStore`, `ActivityStore`, `AttachmentStore`, `UsageStore`, `ServiceStore`, `ServicePlanStore`, `UserStore`, `ShopStore`, `Store`)  |
| `helpers.rs`             | Factory functions: `test_part()`, `test_session()`, `test_activity()`, `test_store()`, constants for `PartTypeId` values                                                                |
| `private_tests.rs`       | Tests for `Attachment::new()`, `calculate_usage()`, `add_details()`, `usage()`                                                                                                          |
| `free_function_tests.rs` | Tests for `attach_one()`, `shift_subparts()`, `subattachments()`, `subparts()`                                                                                                          |
| `public_api_tests.rs`    | Tests for `attach_assembly()`, `detach_assembly()`, `dispose_assembly()`, `recover_assembly()`, `is_attached()`, `register_activity()`, `activities_by_part()`, `for_part_with_usage()` |
| `invariant_tests.rs`     | Timeline invariants, usage conservation, SumHash correctness, ownership rules                                                                                                           |
| `integration_tests.rs`   | Bike assembly lifecycle, gear swap, part replacement, subpart cascade scenarios                                                                                                         |

### Module Declaration

In `backend/domain/src/entities.rs`, add:

```rust
#[cfg(test)]
mod attachment;
```

Or alternatively, include it from `lib.rs`:

```rust
#[cfg(test)]
mod entities::attachment;
```

---

## Priority Ordering

Given the complexity, tests should be implemented in this priority order:

| Priority | Phase                            | Rationale                                      |
| -------- | -------------------------------- | ---------------------------------------------- |
| P0       | 1 (MemStore)                     | Foundation — no tests without this             |
| P0       | 4.1 (attach_assembly validation) | Prevents invalid states at the API boundary    |
| P1       | 4.1.2 (Basic attach)             | Core functionality                             |
| P1       | 3.1 (attach_one)                 | Critical logic shared by all attach operations |
| P1       | 4.2 (detach_assembly)            | Mirror of attach, equally critical             |
| P2       | 2 (Private methods)              | Internal correctness                           |
| P2       | 4.3 (dispose/recover)            | Important but less frequently exercised        |
| P2       | 5 (Invariants)                   | Safety net for complex interactions            |
| P3       | 3, 4.4-4.7 (Remaining)           | Edge cases, utility functions                  |
| P3       | 6 (Integration)                  | Valuable but more expensive to maintain        |

---

## Notes

### Time Handling

- All timestamps use `OffsetDateTime`
- The `round_time()` function from [`lib.rs`](backend/domain/src/lib.rs:40) normalizes times before processing
- "Still attached" is represented by `detached == MAX_TIME`

### PartTypeId Values

- Part types are defined in [`objects.rs`](backend/domain/src/entities/types/objects.rs)
- Tests should use actual PartTypeId values (not arbitrary integers)
- The `subtypes()` method on PartTypeId determines which parts can be subparts

### Error Types

- `Error::BadRequest` — invalid input (hook not compatible, gear type wrong)
- `Error::Forbidden` — ownership violations
- `Error::NotFound` — part not attached at specified time
- `Error::Conflict` — disposal conflict (part still attached)

### Service Integration

- The attachment module calls `Service::recalculate()` and `Service::get_usageids()`
- These require the `ServiceStore` trait in the MemStore
- For Phase 1-2 tests, service store methods can return empty vectors
- For Phase 4+ tests, a minimal service implementation may be needed
