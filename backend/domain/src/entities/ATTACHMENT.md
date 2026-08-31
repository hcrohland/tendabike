# Attachment Entity

## Purpose

The attachment entity models a **temporal, hierarchical relationship** between parts on a bike. An attachment records that a sub-part was attached to a specific gear (e.g., frame) at a particular hook/position during a time interval. This is the core data structure for tracking **part installation/removal history** and computing **usage-based service schedules**.

---

## Core Data Structure

### `Attachment` (`domain/src/entities/attachment.rs:39-55`)

| Field | Type | Description |
|-------|------|-------------|
| `part_id` | `PartId` | The sub-part being attached (e.g., a chain) |
| `attached` | `OffsetDateTime` | When the part was installed |
| `gear` | `PartId` | The parent gear it is attached to (e.g., the bike frame) |
| `hook` | `PartTypeId` | The attachment point/hook on the gear (type ID of where it mounts) |
| `detached` | `OffsetDateTime` | When the part was removed — `MAX_TIME` (year 9100) means "still attached" |
| `usage` | `UsageId` | UUID referencing a usage record that aggregates Strava activity metrics |

**Composite primary key**: `(part_id, attached)` — the timeline is append-only via new rows.

### `AttachmentDetail` (`attachment.rs:61-74`)

Extends `Attachment` with denormalized `name` and `what` (type ID) fields to simplify client serialization, especially for parts that were sold/disposed.

---

## Part Type System & Hierarchy

Attachments are governed by the **PartType** registry (`domain/src/entities/types.rs:57-71`) which defines:

- **`id`** — unique type identifier (e.g., chain = 4)
- **`main`** — which gear type this part belongs to (always 1 = Bike for bike components)
- **`hooks`** — which part types this can attach to

The hierarchy looks like:

```
Bike (main=1, hooks=[])
  ├── Front Wheel (hooks=[1→Bike])
  │     └── Tire (hooks=[2→FrontWheel, 5→RearWheel])
  │     └── Brake Rotor (hooks=[2,5→Wheels])
  ├── Rear Wheel (hooks=[1→Bike])
  │     └── Cassette (hooks=[5→RearWheel])
  │     └── Brake Rotor (hooks=[2,5→Wheels])
  ├── Chain (hooks=[1→Bike])
  ├── Front Brake (hooks=[1→Bike])
  ├── Rear Brake (hooks=[1→Bike])
  │     └── Brake Pad (hooks=[7→FrontBrake, 8→RearBrake])
  ├── Crank (hooks=[1→Bike])
  │     └── Chainring (hooks=[13→Crank])
  │           └── Pedal (hooks=[13→Crank])
  ├── Seat Post (hooks=[1→Bike])
  │     └── Saddle (hooks=[10→SeatPost])
  ├── Fork (hooks=[1→Bike])
  └── Rear Shock (hooks=[1→Bike])
```

---

## Attach/Detach Operations

### `attach_assembly()` (`attachment.rs:439-514`)

The main public function for installing a part. It:

1. **Validates** that the part type's `hooks` list includes the target gear type
2. **Detaches** any existing attachment of this part (if already attached elsewhere)
3. **Detaches** any part currently occupying the target hook on the gear (replacement behavior)
4. Calls `attach_one()` which:
   - Finds successor attachments on the same hook (time-based overlap detection)
   - Merges adjacent attachments (same part, same gear/hook, contiguous time ranges)
   - Creates the new attachment with correct `detached` boundary from successor
   - Copies ownership/shop context from gear to the new part via `set_owner_and_shop()`
5. If `all=true`, **reattaches all subparts** that were detached with the main part

### `detach_assembly()` (`attachment.rs:516-531`)

Removes a part from its gear at a given time. With `all=true`, also detaches all subparts recursively via `shift_subparts()`.

### `shift()` (`attachment.rs:113-123`)

Moves a part from one gear to another — effectively detach + reattach in one atomic operation.

### `dispose_assembly()` / `recover_assembly()` (`attachment.rs:533-601`)

For soft-deleting parts — checks no active attachments exist, then marks the part as disposed. Subparts can be detached or also disposed depending on the `all` flag.

---

## Usage Recalculation

Attachments are the bridge between **Strava cycling activities** and **part wear/service schedules**:

1. **`calculate_usage()`** (`attachment.rs:96-103`) — Finds all `Activity` records where `activity.start` falls within `[attached, detached)` for this attachment's gear and aggregates their metrics (time, distance, climb, energy) into a single `Usage`.

2. **`create()`** — When an attachment is created:
   - Calculates aggregated usage from activities covering the attach period
   - Updates the part's `last_used` timestamp
   - Recalculates all pending service usages for this part via `Service::recalculate()`
   - Stores everything atomically

3. **`register_activity()`** (`attachment.rs:258-296`) — Called when a new Strava activity is imported. Finds all parts currently attached to the bike's gear at the activity start time and accumulates usage counters on each.

---

## `SumHash` Aggregation

`SumHash` (`domain/src/entities/summary.rs:57-66`) is a temporary accumulator used across all CRUD operations that modify multiple entities simultaneously:

```rust
struct SumHash {
    activities: HashMap<ActivityId, Activity>,
    parts: HashMap<PartId, Part>,
    atts: HashMap<String, AttachmentDetail>,  // key = "{part_id}{attached}"
    uses: HashMap<UsageId, Usage>,
    servs: HashMap<ServiceId, Service>,
    plans: HashMap<ServicePlanId, ServicePlan>,
    shops: HashMap<ShopId, Shop>,
    users: HashMap<UserId, UserPublic>,
}
```

All multi-entity operations (attach, detach, dispose) accumulate changes into a `SumHash` which is then converted into a `Summary` response containing all affected entities.

---

## SQL Storage Layer

### Table (`sqlx/migrations/...up.sql:104-117`)

```sql
CREATE TABLE attachments (
    part_id integer REFERENCES parts(id),
    attached timestamp with time zone,     -- PART OF PK
    gear integer NOT NULL REFERENCES parts(id),
    hook integer NOT NULL,                  -- PartTypeId value
    detached timestamp with time zone NOT NULL,
    usage uuid NOT NULL DEFAULT gen_random_uuid(),
    PRIMARY KEY (part_id, attached)         -- temporal primary key
);

-- Indexes: attachments_gear_idx (gear), attachments_time_range_idx (attached, detached)
```

### Key store methods (`sqlx/src/store/attachment.rs`)

| Method | Description |
|--------|-------------|
| `attachment_create()` | Insert a new attachment row |
| `delete()` | Delete by `(part_id, attached)` composite key |
| `attachment_get_by_part_and_time()` | Find active attachment at a point in time (with `FOR UPDATE` lock) |
| `attachment_get_by_gear_and_time()` | Find all parts attached to a gear at a given time |
| `attachments_all_by_part()` | Full timeline history for a part |
| `attachment_find_successor()` | Find the next attachment on the same hook (for overlap detection) |
| `attachment_find_part_attached_already()` | Find immediately preceding attachment (for merging) |
| `attachment_find_later_attachment_for_part()` | Find the next attachment of this part after a given time |
| `assembly_get_by_types_time_and_gear()` | Find attachments matching multiple type IDs on a gear at a time |

---

## API Endpoints

### `POST /api/attach` (`axum/src/domain/attachment.rs:34-56`)

Attaches a part to a gear at a specific time and hook.

### `POST /api/detach` (`axum/src/domain/attachment.rs:58-74`)

Detaches a part from its gear at a specific time.

### Request body (`axum/src/domain/attachment.rs:23-31`)

```rust
struct AttachEvent {
    gear: PartId,              // target gear
    part_id: PartId,           // the part to attach/detach
    time: OffsetDateTime,      // operation timestamp
    hook: PartTypeId,          // which hook on the gear
    all: bool,                 // detach entire assembly including subparts
}
```

---

## Query Methods

| Function | Description |
|----------|-------------|
| `subparts(part_id, time)` | Returns `PartId`s of all parts currently attached to this part at `time` |
| `subattachments(part, gear, time)` | Returns `Attachment` records for subparts of a part on a given gear |
| `for_part_with_usage(part_id)` | Returns all attachment timeline entries with denormalized details + usage records |
| `activities_by_part(part, begin, end)` | Returns all activities between the first attachment and last detachment in the time range |
| `is_attached(part_id, time)` | Simple predicate checking if a part is attached at a point in time |

---

## Frontend Model

`frontend/src/lib/attachment.ts:5-53`

```typescript
class Attachment {
  part_id: number;      // sub-part ID
  attached: Date;       // installed date
  gear: number;         // parent gear ID
  hook: number;         // hook type
  detached: Date;       // removed date (or far future)
  what: number;         // part type ID (denormalized)
  name: string;         // part name (denormalized)
  idx: string;          // unique key = "{part_id}/{attached_timestamp}"
  usage: string;        // UsageId UUID
  
  isAttached(time?: Date): boolean    // is this part on the bike at time?
  isDetached(): boolean               // has been removed?
  isEmpty(): boolean                  // detached == attached (deleted)
  activities(acts: Map<Activity>): ... // filter activities in attachment period
}
```

### Helper Functions

| Function | Description |
|----------|-------------|
| `att_at_hook(gear, what, hook, atts)` | Find attachment for a part at a specific hook right now |
| `part_at_hook(gear, what, hook, atts)` | Find part ID at a specific hook, or return the gear if none |
| `attachment_for_part(part, atts, time)` | Return attachment for part at a given time, or undefined |
| `attachees_for_gear(gear, atts)` | Return all parts currently attached to a gear |

---

## Design Patterns & Key Behaviors

1. **Append-only timeline** — no updates to existing attachment rows; removal is done by inserting a new row with `detached` set. This preserves full historical accuracy.

2. **Automatic time boundary management** — `attach_one()` automatically calculates when the new attachment ends based on successor attachments on the same hook, and when preceding adjacent attachments end (for merging).

3. **Ownership propagation** — when a part is attached to a gear, its `owner` and `shop` are copied from the gear via `PartId::set_owner_and_shop()`.

4. **Hook validation** — attach operations check that the part type's `hooks` list includes the target gear type, preventing invalid physical configurations.

5. **Usage assembled from activities** — usage metrics are not manually entered; they are derived by matching Strava activity timestamps against attachment time windows.

---

## Tests

Test coverage lives in `domain/src/entities/attachment.rs:617-3443` with ~15+ tests across:

| Category | Tests | What they cover |
|----------|-------|-----------------|
| `new()` | 3 | Struct creation, default/explicit detached time |
| `subparts()` | 3 | Empty list, children at time, exclusion of detached parts |
| `for_part_with_usage()` | 3 | Empty results, single attachment with details, timeline entries |
| `detach_assembly()` | 3 | Detach part, detach with all=true, set detached time |
| `shift()` | 1+ | Moving parts between gears at different times |

**Test infrastructure**:
- `MemStore` in-memory implementation of all store traits
- `TestSession` with user/shop context
- Fixtures: `fixture_basic_part()`, `fixture_assembly()`, `fixture_timeline()`, `fixture_concurrent_parts()`
- Part type constants: 17 uppercase constants in `test_support/part_type_ids`

---

## Key Files

| File | Role |
|------|------|
| `domain/src/entities/attachment.rs` | Core entity, operations, and unit tests |
| `domain/src/traits/attachment.rs` | Store trait interface (10 methods) |
| `domain/src/entities/types.rs` | PartType definitions with hooks |
| `domain/src/entities/types/objects.rs` | Static part type registry (20 types) |
| `domain/src/entities/summary.rs` | SumHash aggregation for multi-entity operations |
| `domain/src/entities/service.rs` | Service recalculation triggered by attachments |
| `sqlx/src/store/attachment.rs` | PostgreSQL storage implementation |
| `sqlx/migrations/20250101000000_initial_schema.up.sql` | Database schema |
| `axum/src/domain/attachment.rs` | REST API handlers |
| `frontend/src/lib/attachment.ts` | TypeScript entity class and helpers |
