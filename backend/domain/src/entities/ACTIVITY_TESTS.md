# Activity Test Plan

## Overview

Activities are the primary source of usage data in TendaBike. They represent cycling (and other) sessions tracked by Strava or CSV import, and they propagate their metrics (time, distance, climb, descend, energy) through the attachment system to all parts that were in use at the time.

### Key Flow
```
Activity (from Strava/CSV) 
  → Activity.register() 
    → Attachment::register_activity(gear, time, usage, store)
      → Find all attachments of gear at activity start time
      → Update usage for each attached part
      → Update usage for the gear itself
      → Call Service::get_usageids() for service linkage
```

---

## Pre-requisite: MemStore Infrastructure

ActivityMemStore is already implemented (`mem_activity.rs`):
- `activity_create`, `activity_read_by_id`, `activity_update`, `activity_delete` ✓
- `activities_find_by_gear_and_time` ✓
- `get_all`, `get_by_user_and_time` ✓
- `activity_set_gear_if_null`, `activity_get_really_all` ✓

No new infrastructure needed for activity tests — all store methods are in place.

---

## Suite 1: Activity — Usage Extraction (5 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-01 | `activity_usage_returns_all_metrics` | `usage()` maps all Option<i32> fields to Usage, duration→unused, gear not included |
| A-02 | `activity_usage_defaults_time_to_zero_when_none` | `time=None` → `Usage.time == 0` (not moving_time, not duration) |
| A-03 | `activity_usage_defaults_descend_to_climb` | `descend=None` → `Usage.descend == climb` (the fallback rule from the source code) |
| A-04 | `activity_usage_all_defaults_to_zero` | All Option fields are None → Usage is all zeros with count=1 |
| A-05 | `activity_usage_preserves_all_some_values` | All Option fields Some → Usage matches exactly (time, distance, climb, descend, energy) |

---

## Suite 2: Activity — CRUD Operations (10 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-06 | `activityid_new_creates_id` | `ActivityId::new(42)` → stringifies as `"42"` |
| A-07 | `activityid_read_returns_activity` | Create activity → read by ID → all fields match (user_id, name, start, etc.) |
| A-08 | `activityid_read_optional_returns_none_for_missing` | Read non-existent ID → `Ok(None)` |
| A-09 | `activityid_read_rejects_cross_user` | User 2 tries to read User 1's activity → `Error::Forbidden` |
| A-10 | `activity_upsert_creates_new_activity` | Insert new activity → Summary has 1 activity, stored in MemStore |
| A-11 | `activity_upsert_calls_register_on_create` | After upsert, the gear's usage and all attached parts' usages are updated with the activity metrics |
| A-12 | `activity_upsert_updates_existing` | Same ID as existing → calls `replace()` which unregisters old + registers new |
| A-13 | `activity_update_returns_summary` | Update activity → Summary contains 1 updated activity entry |
| A-14 | `activity_delete_unregisters_usage` | Delete activity → all part usages are decremented (negative Usage values) |
| A-15 | `activity_delete_returns_zeroed_activity` | After delete, activity in Summary has `gear=None`, `time/duration=0`, all metrics `None` |

---

## Suite 3: Activity — Registration with Parts and Attachments (8 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-16 | `activity_register_no_gear_does_not_update_parts` | Activity with `gear=None` → usage NOT propagated to any part or attachment |
| A-17 | `activity_register_with_gear_updates_gear_usage` | Activity with `gear=G01` → gear's usage incremented by activity metrics |
| A-18 | `activity_register_updates_all_attached_parts` | Gear has 3 attached sub-parts → all 3 parts AND the gear itself get usage updated |
| A-19 | `activity_register_uses_attachment_timeline` | Activity at T=50 → only finds attachments where `attached <= 50 && detached > 50` |
| A-20 | `activity_register_skips_detached_parts` | Part detached before activity start → NOT included in usage propagation |
| A-21 | `activity_register_no_attachments_only_updates_gear` | Gear with no sub-parts attached → only the gear's usage is updated, parts list has just gear |
| A-22 | `activity_upsert_replaces_gear_changes_affected_parts` | Old activity on gear G1, new has gear G2 → old parts decremented, new parts incremented |
| A-23 | `activity_delete_decrements_all_affected_parts` | Delete activity → all parts that received usage from this activity get decremented |

---

## Suite 4: Activity — Find by Gear and Time (5 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-24 | `activity_find_returns_matching_gear_in_range` | Activity with gear=G01 at T=1700000000 → find(G01, 1699999000, 1700001000) returns it |
| A-25 | `activity_find_excludes_activities_without_gear` | Activity with `gear=None` → NOT returned by `find(gear)` |
| A-26 | `activity_find_excludes_activities_outside_range` | Activity at T=1700100000 → NOT found when searching 1699999000-1700001000 |
| A-27 | `activity_find_returns_multiple_in_range` | 3 activities for same gear in range → all returned, ordered as stored |
| A-28 | `activity_find_empty_for_gear_no_activities` | Gear with zero activities → `find()` returns empty Vec |

---

## Suite 5: Activity — get_all and categories (4 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-29 | `activity_get_all_returns_user_activities` | Create 3 activities for user U1 → get_all(U1) returns exactly 3 |
| A-30 | `activity_get_all_excludes_other_users` | User U1 has 2 activities, U2 has 2 → get_all(U1) returns only U1's |
| A-31 | `activity_get_all_empty_for_no_activities` | User with no activities → returns empty Vec |
| A-32 | `activity_categories_returns_unique_part_types` | 3 activities with types RIDE(1), RUN(3), RIDE(1) → categories returns {PartTypeId(Bike), PartTypeId(RunningShoe)} |

---

## Suite 6: Activity — replace() (Updation Logic) (5 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-33 | `replace_subtracts_old_activity_usage` | Old activity registered → after replace, old usage is subtracted via `Factor::Sub` |
| A-34 | `replace_adds_new_activity_usage` | After subtracting old, new activity is registered via `Factor::Add` |
| A-35 | `replace_changes_affected_parts` | Old activity on gear G1, new on gear G2 → old parts decremented, new parts incremented |
| A-36 | `replace_same_gear_updates_single_affected_set` | Same gear before/after → parts receive net usage change (new - old) |
| A-37 | `replace_preserves_activity_id` | After replace, the activity retains its original ID (not a new one) |

---

## Suite 7: Activity — set_default_part (6 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-38 | `set_default_part_assigns_gear_to_matching_activities` | Activities with `gear=None` and matching type get assigned the default gear |
| A-39 | `set_default_part_only_affected_matching_types` | Activities of different type → NOT assigned the default gear |
| A-40 | `set_default_part_does_not_override_existing_gear` | Activity already has gear → NOT changed by set_default_part |
| A-41 | `set_default_part_recalculates_usage` | After gear assignment, activity.register(Factor::Add) propagates usage to new gear and attachments |
| A-42 | `set_default_part_empty_returns_zero_usage` | No activities match → Summary.usages is empty, no errors |
| A-43 | `set_default_part_requires_ownership` | User tries to set default gear for other user's activities → fails with Forbidden |

---

## Suite 8: Activity — rescan_all (3 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-44 | `rescan_all_deletes_all_usages_first` | Before rescanning, all usage records are wiped via `Usage::delete_all()` |
| A-45 | `rescan_all_reregisters_every_activity` | After rescan, every activity has been re-registered with Factor::Add |
| A-46 | `rescan_all_produces_same_usage_as_original` | Usage totals after rescan match the usage totals before any modifications (idempotent re-computation) |

---

## Suite 9: Activity — CSV Import (descend parsing) (6 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-47 | `csv2descend_parses_german_date_format` | "2023-11-15 08:30:00" → parsed correctly as OffsetDateTime |
| A-48 | `csv2descend_parses_english_title_field` | "Titel" and "Title" aliases both parsed as activity name |
| A-49 | `csv2descend_parses_descend_variants` | "Negativer Höhenunterschied", "Abstieg gesamt", "Total Descent" → all recognized |
| A-50 | `csv2descend_skips_german_decimal_format` | "1.234" descend → stripped of dots, parsed as 1234 |
| A-51 | `csv2descend_returns_good_and_bad_lists` | 3 valid + 1 invalid record → good=[3], bad=[1] with descriptions |
| A-52 | `csv2descend_calls_match_and_update_for_each_record` | Each valid CSV row → stored activity updated with climb/descend via `match_and_update()` |

---

## Suite 10: Activity — Usage Vector Operations (4 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-53 | `usage_add_two_usages` | `u1 + u2` → all fields summed (time, distance, climb, descend, energy, count) |
| A-54 | `usage_add_vec_adds_single` | `vec + &usage` → each element in vec incremented by usage |
| A-55 | `usage_subtracts_correctly` | `u1 - u2` → all fields subtracted (used for unregister) |
| A-56 | `usage_negation_produces_inverted` | `-usage` → all fields negated (used for Factor::Sub) |

---

## Suite 11: Activity — Integration with Services (5 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-57 | `activity_register_updates_service_usage_references` | During `Attachment::register_activity()`, `Service::get_usageids()` is called to link services |
| A-58 | `activity_delete_affects_service_usage_links` | After deleting an activity, services that referenced its usage should reflect the change |
| A-59 | `activity_register_main_part_ignores_attachment_timeline` | For main parts (is_main=true), activities are found by gear regardless of attachment periods |
| A-60 | `activity_register_sub_part_uses_attachment_periods` | For sub-parts (is_main=false), activities only counted during attachment windows |
| A-61 | `activity_recalculate_after_new_attachment_added` | After attaching a new sub-part to the gear, subsequent activities include the new part in usage |

---

## Suite 12: Activity — Edge Cases and Error Handling (6 tests)

| ID | Test Name | What It Validates |
|----|-----------|-------------------|
| A-62 | `activity_delete_missing_activity_returns_not_found` | Delete non-existent activity → Error::NotFound |
| A-63 | `activity_update_missing_activity_returns_not_found` | Update non-existent activity → Error::NotFound |
| A-64 | `activity_upsert_preserves_original_id` | Insert activity with custom ID → stored with same ID, not auto-generated |
| A-65 | `activity_delete_preserves_activity_record` | After delete, the Activity row still exists (with zeroed fields) but usage is cleaned up |
| A-66 | `activity_with_zero_duration_still_registered` | Activity with duration=0 but all metrics present → usage still propagated correctly |
| A-67 | `activity_with_only_climb_no_other_metrics` | Only climb is Some → Usage defaults other Option fields to 0, descend=climb |

---

## Summary

| Suite | Scope | Count |
|-------|-------|-------|
| 1 — Usage Extraction | Basic usage mapping from Activity fields | 5 |
| 2 — CRUD Operations | Standard create/read/update/delete with ownership | 10 |
| 3 — Registration with Parts/Attachments | Core propagation through attachment system | 8 |
| 4 — Find by Gear and Time | Time-range queries for service usage calculation | 5 |
| 5 — get_all and categories | User-scoped queries and type classification | 4 |
| 6 — replace() (Updation Logic) | Old-new swap semantics for activity edits | 5 |
| 7 — set_default_part | Batch gear assignment for orphaned activities | 6 |
| 8 — rescan_all | Full re-computation of all usage metrics | 3 |
| 9 — CSV Import (descend) | German-format CSV parsing and import | 6 |
| 10 — Usage Vector Ops | Arithmetic on Usage (add, sub, negation) | 4 |
| 11 — Integration with Services | Service linkage via usage references | 5 |
| 12 — Edge Cases and Errors | Boundary conditions, ownership, error handling | 6 |
| **Total** | | **67 tests** |

---

## Test Patterns to Follow

- Use `test_support::{MemStore, TestSession}` for store and session setup
- Helper functions: `test_user()`, `test_session()`, `sample_activity()` (already defined in activity.rs), `sample_purchase_date()`
- Use `#[tokio::test]` for all async tests, `#[test]` for sync
- Return `TbResult<()>` from tests for error propagation
- Use fixed timestamps via `time::OffsetDateTime::from_unix_timestamp()` for deterministic tests
- Create parts with `Part::create()` before testing activity-gear associations
- Use attachment fixtures (`fixtures::fixture_attached_part()`) for testing registration through the attachment timeline

## Key Test Setup Patterns

```rust
// Create a gear with attached sub-parts for attachment timeline tests
let gear = Part::create("Bike", "Trek", "Domane", bike_type, None, purchase_date, "", &session, &mut store).await?;
let chain = Part::create("Chain", "Shimano", "CN-HG50", chain_type, Some(gear.id), purchase_date, "", &session, &mut store).await?;
let att = Attachment::create(chain.id, gear.id, attach_time, detached_future, PartTypeId::from(5), &session, &mut store).await?;
```

## Dependencies Between Suites

- **Suite 1** (Usage Extraction) must pass before Suite 2+ — usage is the atomic unit of propagation
- **Suite 10** (Usage Vector Ops) must pass before Suite 2+ — Usage add/sub/neg are used throughout
- **Suite 3** (Registration) depends on proper MemStore for attachments — `mem_service.rs` is NOT needed, but attachment storage works since it's already implemented
- **Suite 11** (Integration with Services) requires the Service entity tests to be understood — `Service::get_usageids()` is called during activity registration
- **Suite 7** (set_default_part) and **Suite 8** (rescan_all) are standalone but depend on Suite 2 (CRUD) being functional
