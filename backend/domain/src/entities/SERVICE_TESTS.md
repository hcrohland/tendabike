# Service & ServicePlan Test Plan

## Pre-requisite: MemStore Infrastructure

Before writing entity tests, the in-memory store stubs need implementation:

**`mem_service.rs`** — currently all methods `todo!()`. Needs:
```rust
impl ServiceStore for MemStore {
    async fn create(...) → insert into self.services, return clone
    async fn get(...) → find by ServiceId in self.services
    async fn update(...) → find, mutate, return clone
    async fn delete(...) → remove by id, count deleted; also cleanup successor chains
    async fn services_delete(...) → bulk delete
}
```

**`mem_serviceplan.rs`** — needs:
```rust
impl ServicePlanStore for MemStore {
    async fn create(...) → insert into self.service_plans
    async fn get(...) → find by ServicePlanId
    async fn plan_update(...) → find, mutate
    async fn delete(...) → remove by id
    async fn serviceplans_delete(...) → bulk delete
    async fn by_part(...) → filter self.service_plans by .part == Some(part_id)
    async fn by_user(...) → filter self.service_plans by .uid == uid
}
```

**`mem_activity.rs`** — needs `activities_find_by_gear_and_time()` implementation:
```rust
fn activities_find_by_gear_and_time(&mut self, gear: PartId, begin: OffsetDateTime, end: OffsetDateTime)
```

---

## Suite 1: Service — Create & Read (10 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| S-01 | `service_id_new_creates_uuid` | `ServiceId::new()` produces a v7 UUID (format check) |
| S-02 | `service_create_with_valid_data` | Create succeeds, returns correct `part_id`, `time`, `name`, `notes`, empty `plans` |
| S-03 | `service_create_usage_is_created` | A Usage record is created and linked via `service.usage` |
| S-04 | `service_create_returns_summary_with_service_and_usage` | `Summary.services.len() == 1`, `Summary.usages.len() == 1` |
| S-05 | `service_create_usage_main_part_aggregates_all_activities` | For a main part (bike), `calculate_usage()` sums ALL activities from `MIN_TIME` to service time |
| S-06 | `service_create_usage_sub_part_aggregates_activities_during_attachment` | For a sub-part (chain), `calculate_usage()` sums activities from `Attachment::activities_by_part()` during attached periods |
| S-07 | `service_create_usage_zero_with_no_activities` | Service on a part with zero activities → `Usage { time:0, distance:0, climb:0, ... }` |
| S-08 | `serviceid_get_returns_stored_service` | Create → Get by ID → assert all fields match |
| S-09 | `serviceid_get_returns_not_found_for_missing` | Get non-existent ID → `Err(Error::NotFound)` |
| S-10 | `services_by_part_returns_only_matching` | Create services for parts A, B, C → `services_by_part(A)` returns only A's services |

## Suite 2: Service — Update & Delete (7 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| S-11 | `service_update_recalculates_usage` | After updating name/notes, `calculate_usage()` is called and usage record updated |
| S-12 | `service_update_preserves_usage_reference` | `self.usage` field is NOT changed by update (preserved from original) |
| S-13 | `service_update_requires_ownership` | User who doesn't own the part tries to update → fails with error |
| S-14 | `service_delete_removes_service_and_usage` | Delete → `get()` returns error, usage record removed from store |
| S-15 | `service_delete_rewires_predecessor_chains` | Delete middle of chain → all predecessors point `successor = None` instead of deleted id |
| S-16 | `service_delete_no_rewire_when_no_predecessors` | Delete leaf node (no predecessors) → no updates, chain intact |
| S-17 | `service_delete_by_partid_checks_ownership` | Delete via `ServiceId::delete()` checks part ownership same as update |

## Suite 3: Service — Redo & Successor Chains (7 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| S-18 | `redo_earlier_time_creates_new_entry_as_successor` | Redo Jan 1 service at Dec 15 → new service with `successor = original.id` |
| S-19 | `redo_later_time_inserts_between_chain` | Chain: #1→#2. Redo #1 at Feb 1 → new entry #new, `old.successor = new.id`, `#2.successor = None` |
| S-20 | `redo_preserves_name_and_notes_from_original` | Original has name="Clean", notes="Lube" → redo produces same name/notes |
| S-21 | `redo_preserves_plans_from_original` | Original linked to plan P1 → redo service also has `plans = [P1]` |
| S-22 | `redo_recalculates_usage_for_new_service` | Redo service at different time → new usage computed from activities in new window |
| S-23 | `redo_requires_ownership` | Redo service on part owned by different user → ownership check fails |
| S-24 | `redo_returns_summary_with_new_service_and_updated_old` | Summary includes both the new service and the modified old service |

## Suite 4: Service — Usage Calculation (6 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| S-25 | `service_calculate_usage_main_part_finds_activities_by_gear` | For part where `is_main() == true`, calls `Activity::find(gear, MIN_TIME, time)` |
| S-26 | `service_calculate_usage_sub_part_finds_activities_by_attachment_period` | For non-main parts, calls `Attachment::activities_by_part(part, MIN_TIME, time)` |
| S-27 | `service_calculate_usage_aggregates_multiple_activities` | 3 activities between MIN_TIME and service.time → `Usage.count == 3`, sums time/distance/climb |
| S-28 | `service_recalculate_filters_by_attach_time` | `Service::recalculate(part, attach_time)` only recalculates services where `attach <= service.time` |
| S-29 | `service_recalculate_after_detach_stale_services` | If a part is detached at time T, services after T should be recalculated with no new activities from the attachment |
| S-30 | `service_recalculate_handles_empty_service_list` | Part with no services → `recalculate()` returns empty Vec, no errors |

## Suite 5: Service — Integration with Attachments (4 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| S-31 | `service_recalculated_on_attachment_create` | Create attachment → services on that part after attach_time get recalculated via `Service::recalculate()` |
| S-32 | `service_recalculated_on_attachment_delete` | Delete attachment → services after detach_time recalculated (usage may change since activities excluded) |
| S-33 | `service_recalculate_updates_usage_vec_in_place` | Multiple services recalculated in batch → all updated via single `Usage::update_vec()` call |
| S-34 | `attach_assembly_updates_service_usage` | Full attach_assembly flow → returned Summary includes recalculated service usages |

## Suite 6: ServicePlan — CRUD (12 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| SP-01 | `service_plan_create_specific_part` | Create with `part = Some(id)` → stored, `uid = None`, `what = part_type_of_that_part` |
| SP-02 | `service_plan_create_generic_plan` | Create with `part = None` → stored, `uid = Some(current_user_id)` |
| SP-03 | `service_plan_create_sets_what_to_part_type` | For specific part, `what` is auto-set to the part's `PartTypeId` |
| SP-04 | `service_plan_create_with_thresholds` | Create with `hours = Some(60)` → persisted, accessible on read |
| SP-05 | `service_plan_get_returns_stored_plan` | Create → Get by ID → all fields match |
| SP-06 | `serviceplan_by_part_returns_matching_plans` | Create plans for parts A and B → `by_part(A)` returns only A's plans |
| SP-07 | `serviceplan_by_user_returns_generic_plans` | Create generic plans for user 1 and user 2 → `by_user(1)` returns only user 1's |
| SP-08 | `service_plan_update_preserves_immutable_fields` | Update name but not `part`, `what`, `hook`, `uid` → immutable fields unchanged |
| SP-09 | `service_plan_update_requires_ownership_specific` | Update specific plan → checks part ownership via `part.checkuser()` |
| SP-10 | `service_plan_update_requires_ownership_generic` | Update generic plan → checks `uid == user.user_id()` |
| SP-11 | `service_plan_delete_removes_plan` | Delete → not found on read, services returned (currently empty per `reset_plan()`) |
| SP-12 | `service_plan_delete_noop_on_reset_plan` | Currently `reset_plan()` returns `Ok(vec![])` → delete returns empty service list |

## Suite 7: ServicePlan — Threshold Data Model (4 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| SP-13 | `service_plan_all_thresholds_none_means_active` | All optional threshold fields are `None` → plan is still a valid entry, just time-only or name-only |
| SP-14 | `service_plan_single_threshold_only` | Only `km = Some(500)` → other thresholds remain `None`, valid plan |
| SP-15 | `service_plan_multiple_thresholds` | Set `hours`, `km`, and `climb` → all persisted independently, accessible separately |
| SP-16 | `serviceplan_for_part_returns_empty_when_none` | Query for plans on a part with none → returns `Vec::new()`, not error |

## Suite 8: Service — Edge Cases & Error Handling (8 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| S-35 | `service_create_with_empty_name` | Should succeed (name has no validation in domain layer) |
| S-36 | `service_create_with_very_long_name` | No truncation — `String` stores freely |
| S-37 | `service_successor_chain_single_element` | One service created → its `successor = None`, no predecessors |
| S-38 | `service_successor_chain_two_elements` | Create S1, redo → creates S2 with `S1.successor = Some(S2.id)` |
| S-39 | `service_successor_chain_three_elements` | Create S1, redo → create S2, redo → creates S3, chain: `S1→S2→S3` |
| S-40 | `service_delete_middle_of_long_chain_rewires_all` | Chain of 5 services, delete #3 → S1 and S2 both get `successor = None` |
| S-41 | `service_delete_preserves_unrelated_parts_services` | Delete service on part A → services on part B are untouched |
| S-42 | `service_delete_usage_is_actually_removed` | After delete, `UsageId::read()` on the service's usage should fail |

---

## Summary

| Suite | Scope | Count |
|-------|-------|-------|
| 1 — Create & Read | Basic CRUD, usage on create | 10 |
| 2 — Update & Delete | Modification, removal, chain rewiring | 7 |
| 3 — Redo & Successors | History rewriting, chain management | 7 |
| 4 — Usage Calculation | The core computation logic | 6 |
| 5 — Attachment Integration | Cross-entity behavior | 4 |
| 6 — ServicePlan CRUD | Plan creation, queries, ownership | 12 |
| 7 — Threshold Data Model | Plan field persistence (no eval logic yet) | 4 |
| 8 — Edge Cases | Chain integrity, empty states, boundaries | 8 |
| **Total** | | **54 tests** |

---

## Test Patterns to Follow

- Use `test_support::{MemStore, TestSession}` for store and session setup
- Helper functions: `test_user()`, `test_session()`, fixture parts via `fixtures::fixture_basic_part()` and `fixtures::fixture_attached_part()`
- Use `#[tokio::test]` for async tests, `#[test]` for sync
- Return `TbResult<()>` from tests for error propagation
- Use fixed timestamps via `time::macros::datetime!` for deterministic time-based tests

## Dependencies to Verify in MemStore

1. **`mem_activity.rs`** needs `activities_find_by_gear_and_time()` — currently no activity store trait impl for MemStore exists; must add it
2. **`mem_service.rs`** needs full implementation (all methods `todo!()`)
3. **`mem_serviceplan.rs`** needs full implementation (all methods `todo!()`)
4. **`MemStore.usages`** HashMap must support `UsageId::read()` and `UsageId.delete()` — verify these methods exist on UsageId
