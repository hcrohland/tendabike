# Attachment Entity Test Strategy

## Overview

Comprehensive test plan for the attachment entity in TendaBike. The attachment system manages
when and how bike parts (components) are attached to gears (frame, wheels, etc.) over time.

### Attachment Entity ([`backend/domain/src/entities/attachment.rs`](backend/domain/src/entities/attachment.rs))

- **613 lines** with complex timeline semantics, assembly operations, and usage tracking
- **0 tests** currently
- Key behaviors: auto-merge adjacent attachments, auto-detach when reattaching to same hook,
  hierarchical subpart operations, usage calculation from Strava activities

### MemStore Implementation ([`backend/domain/src/test_support/mem_attachment.rs`](backend/domain/src/test_support/mem_attachment.rs))

- **1 of 12 methods implemented** (`attachments_all_by_part`)
- All other methods are `todo!()` stubs
- Indexed by `(PartId, OffsetDateTime)` in a `HashMap`

### UsageStore ([`backend/domain/src/test_support/mem_usage.rs`](backend/domain/src/test_support/mem_usage.rs))

- All methods are `todo!()` stubs
- Working implementation exists in [`usage.rs:208-240`](backend/domain/src/entities/usage.rs) test module
- Needs extraction to `mem_usage.rs`

### Pattern Reference: Part Entity Tests ([`backend/domain/src/entities/part.rs`](backend/domain/src/entities/part.rs))

- ~20 comprehensive tests using `#[tokio::test]` pattern
- Uses `MemStore` and `TestSession` from test_support
- Follows Arrange-Act-Assert pattern with `-> TbResult<()>` return type

### Dependency Tracking Pattern

Tests should track which parts, attachments, and usages are created/modified using `Summary`
and `SumHash` return values. This enables assertions about cascading effects (e.g., detaching
a parent assembly should detach all subparts).

## Dependencies Between Stores

### Phase 1: MemStore Infrastructure

This phase establishes the test infrastructure by extracting and completing MemStore implementations.

#### 1.1 Extract UsageStore MemStore from usage.rs

Extract the working `UsageStore` implementation from [`usage.rs:208-240`](backend/domain/src/entities/usage.rs)
into [`mem_usage.rs`](backend/domain/src/test_support/mem_usage.rs).

- `update()` - Update usage records by ID
- `get()` - Retrieve usage by ID
- `delete()` - Remove usage record
- `delete_all()` - Bulk delete

#### 1.2 Complete AttachmentStore MemStore Implementation

Complete all 9 remaining methods in [`mem_attachment.rs`](backend/domain/src/test_support/mem_attachment.rs):

- `attachment_create()` - Insert into HashMap
- `delete()` - Remove from HashMap
- `attachment_get_by_gear_and_time()` - Query by gear part_id and start time
  - Filter `attachments.values()` where `a.gear == act_gear && a.attached <= start && a.detached > start`
- `attachment_get_by_part_and_time()` - Query by part_id and attached time
  - Direct HashMap lookup by `(part_id, attached_time)` key
- `assembly_get_by_types_time_and_gear()` - Query subparts
  - Find attachments where part_id matches any type in `types`, at the given gear and time
- `attachment_find_part_of_type_at_hook_and_time()` - Find competing attachment at same hook
  - Find attachment where `gear == gear && hook == hook && attached <= time && detached > time`
- `attachment_find_successor()` - **Critical for timeline logic**
  - Find next attachment for same (part_id, gear, hook) after given time
  - Must search forward in time for same part reattached to same gear at same hook
- `attachment_find_later_attachment_for_part()` - Timeline query
  - Find attachment for same part where attached > time
- `attachment_find_part_attached_already()` - Currently attached part query
  - Find attachment where `part_id == part_id && gear == gear && hook == hook` at given time

### Phase 2: Prepopulated MemStore Fixtures

Create helper functions in a new `test_support/fixtures.rs` module (or add to existing test_support)
to set up predefined MemStore states for complex multi-step scenarios. This avoids building up
state step-by-step in every test and makes tests more readable.

```rust
// Example fixture function signatures:
pub fn fixture_basic_part(session: &TestSession, store: &mut MemStore) -> Part
pub fn fixture_attached_part(session: &TestSession, store: &mut MemStore, part: Part) -> Attachment
pub fn fixture_assembly(session: &TestSession, store: &mut MemStore, main_part: Part) -> (Attachment, Vec<Part>)
pub fn fixture_timeline(session: &TestSession, store: &mut MemStore, attachments: Vec<(Part, Part, OffsetDateTime)>) -> Vec<Attachment>
```

#### 2.1 Fixture: Basic Part Setup

- Creates a user, a main bike part, and a component part ready for attachment

#### 2.2 Fixture: Single Attachment

- Creates a user, gear, part, and an active attachment at a specific time

#### 2.3 Fixture: Assembly with Subparts

- Creates a main part attached to a gear, plus N subparts attached to the main part
- Returns (main_attachment, subparts_attachments, subpart_parts)

#### 2.4 Fixture: Timeline Sequence

- Creates multiple attachments for the same part/gear/hook at different times
- Useful for testing successor/later queries and merge behavior

### Phase 3: Unit Tests for Attachment Entity Methods

Test the `Attachment` impl methods directly using concrete dates and parts.
These are pure unit tests that don't require MemStore - they test the business logic
functions on the `Attachment` struct itself.

#### 3.1 subparts() Helper Function Tests

Test the `subparts()` and `subattachments()` module-level helper functions:

- **`subparts_returns_children_at_time()`** - Given an attachment with subpart IDs, returns
  only those subparts that are attached at the given time (not detached/disposed)
- **`subparts_filters_disposed_parts()`** - Returns empty for disposed subparts
- **`subparts_empty_for_no_children()`** - Returns empty vec when no subpart IDs provided

#### 3.2 Attachment::new() Tests

Test the private constructor (accessible within the same crate via tests):

- **`new_creates_correct_struct()`** - Verify all fields are set correctly
- **`new_calculates_default_end_time()`** - Default `detached` should be far future

#### 3.3 Attachment::for_part_with_usage() Tests

Test the factory method that creates an attachment with calculated usage:

- **`for_part_with_usage_creates_attachment()`** - Creates attachment with correct part_id
- **`for_part_with_usage_sets_default_detached()`** - Sets detached time to far future

#### 3.4 Attachment::detach_assembly() Tests

Test the core detach logic that handles recursive subpart detachment:

- **`detach_assembly_sets_detached_time()`** - Changes detached to given time
- **`detach_assembly_all_true_detaches_subparts()`** - When `all: true`, recursively detaches all subparts
- **`detach_assembly_all_false_keeps_subparts_attached()`** - When `all: false`, only detaches main part
- **`detach_assembly_updates_parent_summary()`** - Returns Summary with all affected parts/usages

#### 3.5 Attachment::shift() Tests

Test the shift operation that moves an attachment to a different gear:

- **`shift_changes_gear_id()`** - Updates gear to new value
- **`shift_creates_new_attachment_at_current_time()`** - Creates new attachment at shift time
- **`shift_detaches_old_attachment()`** - Detaches old attachment at shift time

### Phase 4: Unit Tests for Public API Functions

Test the public module-level functions (`attach_assembly`, `detach_assembly`, `dispose_assembly`,
`recover_assembly`) directly. These are still unit tests (not integration) because they test
individual business logic functions in isolation using MemStore as a simple data store.

#### 4.1 attach_assembly() Tests

**Happy Path:**

- **`attach_assembly_attaches_part_to_gear()`** - Creates attachment record with correct fields
- **`attach_assembly_with_subparts_attaches_all()`** - When `subparts: vec![...]`, attaches all subparts too
- **`attach_assembly_all_true_detach_existing_subparts()`** - When `all: true` and subparts already attached, detaches them first

**Auto-Detach Behavior (not errors):**

- **`attach_assembly_auto_detaches_and_reattaches_same_part()`** - When same part is already attached at same hook,
  automatically detaches old attachment first, then creates new one. Verifies:
  - Old attachment has `detached` time set to current time
  - New attachment has `attached` time set to current time
  - Only one active attachment exists for the part at that time after operation

- **`attach_assembly_auto_detaches_competing_part_at_hook()`** - When a different part is attached
  at the same gear/hook, automatically detaches it first. Verifies:
  - Competing part's attachment has `detached` time set
  - New part is attached at the hook
  - No competing attachments remain active

- **`attach_assembly_merge_adjacent_with_previous()`** - When part was already attached (adjacent timeline),
  the previous attachment is closed and a new one started. Verifies:
  - Previous attachment's `detached` time is updated
  - New attachment starts at the given time

- **`attach_assembly_merge_adjacent_with_subsequent()`** - When part is scheduled for future reattachment,
  the subsequent attachment is removed and current one extended. Verifies:
  - Subsequent attachment is deleted
  - Current attachment's `detached` time is extended

**Summary/SumHash Verification:**

- **`attach_assembly_returns_summary_with_all_parts()`** - Summary contains all parts that were affected
- **`attach_assembly_with_subparts_returns_all_subpart_ids()`** - Subpart part_ids included in summary

#### 4.2 detach_assembly() Tests

**Happy Path:**

- **`detach_assembly_sets_detached_time()`** - Changes attachment's detached time to now
- **`detach_assembly_all_true_detaches_subparts()`** - Recursively detaches all subparts
- **`detach_assembly_returns_summary_with_all_affected()`** - Summary includes parent and subparts

**Edge Cases:**

- **`detach_assembly_already_detached_returns_none()`** - No-op when already detached (detached time in past)
- **`detach_assembly_part_not_found_returns_error()`** - Error when part_id not found in attachments

#### 4.3 dispose_assembly() Tests

**Happy Path:**

- **`dispose_assembly_sets_disposed_at()`** - Marks part and optionally subparts as disposed
- **`dispose_assembly_all_true_disposes_subparts()`** - Recursively disposes all subparts
- **`dispose_assembly_active_attachment_becomes_detached()`** - Any active attachment is detached

**Edge Cases:**

- **`dispose_assembly_already_disposed_keeps_timestamp()`** - Idempotent: keeps existing disposed_at

#### 4.4 recover_assembly() Tests

**Happy Path:**

- **`recover_assembly_clears_disposed_at()`** - Restores part from disposed state
- **`recover_assembly_all_true_recovers_subparts()`** - Recursively recovers all subparts
- **`recover_assembly_returns_restored_part()`** - Returns the recovered Part record

**Edge Cases:**

- **`recover_assembly_not_disposed_returns_error()`** - Error when part was never disposed
- **`recover_assembly_part_not_found_returns_error()`** - Error when part_id not found

### Phase 5: Timeline Query Tests (High Priority)

These tests verify the critical timeline logic that powers attach/detach operations.
Timeline queries must be correct before testing higher-level operations because `attach_assembly`
and `detach_assembly` depend on them.

#### 5.1 attachment_find_successor() Tests

**Happy Path:**

- **`find_successor_returns_next_attachment()`** - Given part A attached 2024-01-01..2024-06-01,
  then reattached 2024-06-01..now, returns the second attachment when queried at 2024-03-01
- **`find_successor_for_different_part_returns_none()`** - Does not return attachments for other parts
- **`find_successor_for_different_gear_returns_none()`** - Does not return attachments to different gears

**Edge Cases:**

- **`find_successor_at_exact_boundary()`** - Query at exact detached time of current attachment
- **`find_successor_no_future_attachment_returns_none()`** - Returns None when part not reattached

#### 5.2 attachment_find_later_attachment_for_part() Tests

**Happy Path:**

- **`find_later_returns_next_for_same_part()`** - Finds any future attachment for the part regardless of gear/hook

**Edge Cases:**

- **`find_later_no_future_returns_none()`** - Returns None when no future attachment exists

#### 5.3 attachment_find_part_attached_already() Tests

**Happy Path:**

- **`find_part_attached_already_returns_active_attachment()`** - Returns attachment when part is currently attached
  to the specified gear at the specified hook

**Edge Cases:**

- **`find_part_attached_already_returns_none_when_not_attached()`** - Returns None when part is not currently attached
- **`find_part_attached_already_returns_none_for_wrong_gear()`** - Returns None when part is attached to different gear

### Phase 6: Integration Tests with Usage Tracking

Test the interaction between attachments, activities, and usages. These tests verify that
usage tracking works correctly through the full call chain.

#### 6.1 Usage Calculation Tests

- **`register_activity_updates_usage()`** - After registering a Strava activity, usage is incremented
- **`register_activity_with_gear_calculation()`** - Usage calculation considers gear association

#### 6.2 Summary Tracking Tests

- **`attach_assembly_tracks_all_usages()`** - Summary contains all affected usage records
- **`detach_assembly_tracks_released_usages()`** - Summary reflects released usage allocations

### Phase 7: Edge Cases and Business Logic

#### 7.1 Merging Adjacent Attachments

- **`attach_merge_creates_seamless_timeline()`** - Two adjacent attachments form continuous period

#### 7.2 Error Cases

- **`attach_assembly_rejects_invalid_part_type()`** - Hook type validation
- **`detach_assembly_nonexistent_part_errors()`** - Error on missing part

#### 7.3 Assembly Operations

- **`attach_assembly_nested_subparts()`** - Subparts with their own subparts
- **`dispose_then_recover_maintains_timeline()`** - Dispose/recover cycle preserves attachment history

## File Changes Summary

| File                             | Action     | Description                                         |
| -------------------------------- | ---------- | --------------------------------------------------- |
| `test_support/fixtures.rs`       | **Create** | Prepopulated MemStore fixture helpers               |
| `test_support/mem_usage.rs`      | **Modify** | Extract from usage.rs, complete all methods         |
| `test_support/mem_attachment.rs` | **Modify** | Complete all 9 remaining methods                    |
| `entities/attachment.rs`         | **Modify** | Add `#[cfg(test)] mod tests` module with ~40+ tests |

## Test Structure Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use time::macros::datetime;

    #[tokio::test]
    async fn attach_assembly_auto_detaches_and_reattaches_same_part() -> TbResult<()> {
        // Arrange: Create fixture state with part already attached
        let session = TestSession::new(UserId::new(1));
        let mut store = MemStore::new();

        let gear = Part::create(&session, /* ... main bike part ... */).await?;
        let part = Part::create(&session, /* ... component part ... */).await?;

        // Part already attached from t1 to now
        let t1 = datetime!(2024-01-01 00:00:00 UTC);
        let t2 = datetime!(2024-06-01 00:00:00 UTC);
        let _first = attach_assembly(&session, part.id, gear.id, &[], true, t2, &mut store).await?;

        // Act: Reattach at t2
        let summary = attach_assembly(&session, part.id, gear.id, &[], true, t2, &mut store).await?;

        // Assert: Old attachment closed, new one started
        let all = store.attachments_all_by_part(part.id).await?;
        assert_eq!(all.len(), 2); // Two timeline entries
        assert_eq!(all[0].detached, t2); // Old one closed at t2
        assert_eq!(all[1].attached, t2); // New one started at t2
        assert!(summary.parts.len() >= 2); // Both old and new in summary
        Ok(())
    }
}
```

## Testing Priorities

1. **MemStore Infrastructure** - Extract UsageStore, complete AttachmentStore (blocking dependency)
2. **Timeline Queries** - `find_successor`, `find_later`, `find_part_attached_already` (critical for attach/detach logic)
3. **Attachment Entity Methods** - `new()`, `for_part_with_usage()`, `detach_assembly()` (business logic)
4. **attach_assembly()** - Verify auto-detach/reattach behavior (most frequently used)
5. **detach_assembly()** - Recursive subpart handling (secondary but important)
6. **dispose/recover_assembly()** - Lifecycle management (less common operations)
7. **Usage Integration** - Ensure usage tracking works correctly

## Risk Assessment

| Risk                                                     | Severity | Mitigation                                                            |
| -------------------------------------------------------- | -------- | --------------------------------------------------------------------- |
| AttachmentStore MemStore takes long to implement         | Medium   | Start with simplest methods first (`attachment_get_by_part_and_time`) |
| Timeline queries may have edge cases with boundary times | Medium   | Test at exact millisecond boundaries                                  |
| Usage calculation depends on Strava activities           | Low      | Mock activity data in MemStore                                        |
| 613-line file may need splitting before/during testing   | Low      | Keep tests in same file for now, refactor later if needed             |

## Success Criteria

- [ ] All 12 AttachmentStore methods implemented in MemStore
- [ ] All 4 UsageStore methods implemented in MemStore
- [ ] ~50+ tests covering all public functions and key business logic
- [ ] `attach_assembly` auto-detach behavior verified (not treated as error)
- [ ] Timeline queries return correct results for all boundary conditions
- [ ] Assembly operations with subparts tested recursively
- [ ] No `todo!()` remaining in MemStore implementations
