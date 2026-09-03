# Service & ServicePlan Entities

## Overview

The service system tracks **maintenance history** and defines **future maintenance obligations**. It sits between the attachment timeline (what parts are physically on the bike) and Strava activity data (how much the bike has been ridden).

```
Attachment (physical timeline)  ──▶  Service (maintenance history)  ──▶  ServicePlan (future obligations)
       │                                      │                                    │
  "I installed a chain"              "I cleaned the drivetrain"        "Service every 500km"
       │                                      │                                    │
       ▼                                      ▼                                    ▼
  Activity matching            Usage back-calculated          Threshold comparison:
  [attached, MAX_TIME]         [MIN_TIME, service.time]       usage > threshold?
```

---

## Service Entity

### Data Model (`domain/src/entities/service.rs:55-75`)

```rust
pub struct Service {
    pub id:          ServiceId,       // UUID v7 (auto-generated)
    pub part_id:     PartId,          // The part that was serviced
    pub time:        OffsetDateTime,  // When the service happened
    pub redone:      OffsetDateTime,  // When superseded by a new service (MAX_TIME = still valid)
    pub name:        String,          // Human-readable name ("Chain cleaning", "Bolt adjustment")
    pub notes:       String,          // Free-form notes
    pub usage:       UsageId,         // Computed usage up to service time (NOT user-entered)
    pub successor:   Option<ServiceId>, // Link to next service on same part (for chaining)
    pub plans:       Vec<ServicePlanId>, // ServicePlans this entry fulfills
}
```

### Lifecycle Methods

| Method | Description |
|--------|-------------|
| `Service::create()` | Create a new service record. Auto-calculates usage from activities. |
| `Service::update()` | Update metadata (name, notes). Usage is recalculated. Ownership checked. |
| `Service::redo()` | Create a new service entry with the same details but at an earlier/later time. Links predecessor via `successor` chain. |
| `Service::delete()` | Deletes the service and its usage record. Rewires successor chains to point to `None`. |

### Usage Calculation (`service.rs:107-115`)

Service usage is **never user-entered**. It is computed from Strava activities:

```rust
if self.part_id.is_main(store).await? {
    // Main parts (bike): find all activities on the bike up to service time
    Activity::find(self.part_id, MIN_TIME, self.time, store).await?
} else {
    // Sub-parts (chain, cassette): find activities during attachment periods
    Attachment::activities_by_part(self.part_id, MIN_TIME, self.time, store).await?
}
// → fold all activities into a Usage record
```

For main parts (the bike itself), this sums all activities ever recorded for that bike up to the service time. For sub-parts (components), this sums activities from periods when the part was attached to a gear — using the attachment timeline to determine *when* the part was in use.

### Successor Chains

Services form chains via the `successor` field:

```
Service #1 (Jan 1) ──┐
    ↓ successor       │ new service at EARLIER time
Service #2 (Mar 1) ──┼── redo() at Feb 1 →
    ↓ successor       │         creates new entry
Service #3 (Jun 1) ──┘         with successor → #1

OR (normal flow):
Service #1 (Jan 1) ──┐
    ↓ successor       │ new service at LATER time
Service #2 (Mar 1) ─── successor = #1 ← updated in redo()
```

`redo()` handles both cases:
- If new time < old time: create a new entry with `successor = old.id` (pushes the old service later in chain)
- If new time > old time: create a new entry, set `old.successor = new.id` (inserts between old and current successor)

### Recalculation Triggers

Services are recalculated whenever attachments change (`service.rs:185-200`):

```
Attachment::create() or Attachment::delete()
       │
       ▼
Service::recalculate(part_id, attach_time)
   ┌── filter: attach_time <= service.time  ─────────┐
   │                                                  │
   ▼                                                  ▼
For each matching service:                       Usage propagation
  calculate_usage()                                    │
       │                                               ▼
       ▼                                        Usage::update_vec() saves all
  Compute usage from activities up to           recalculated services in one batch
  service.time via Activity or Attachment lookup
```

This ensures that if the attachment timeline changes (e.g., a part was reinstalled earlier than recorded), all subsequent service records have their usage metrics updated accordingly.

---

## ServicePlan Entity

### Data Model (`domain/src/entities/serviceplan.rs:38-68`)

```rust
pub struct ServicePlan {
    pub id:        ServicePlanId,   // UUID v7
    pub part:      Option<PartId>,  // Specific part (Some) or generic (None)
    pub what:      PartTypeId,       // The part type (always set)
    pub hook:      Option<PartTypeId>, // Attachment point for generic plans
    pub name:      String,           // "Chain service", "Brake adjustment"
    
    // Thresholds — any non-null threshold triggers a service obligation:
    pub days:      Option<i32>,      // Service every N days since last service
    pub hours:     Option<i32>,      // N cycling hours
    pub km:        Option<i32>,      // N kilometers
    pub climb:     Option<i32>,      // N meters climbed
    pub descend:   Option<i32>,      // N meters descended
    pub rides:     Option<i32>,      // N activities
    pub energy:    Option<i32>,      // N kJ expended (aliased as "kJ" in JSON)
    
    pub uid:       Option<UserId>,   // Owner for generic plans (auto-set to current user)
}
```

### Two Plan Modes

| Mode | `part` field | Use case |
|------|-------------|----------|
| **Specific** | `Some(part_id)` | "Service this exact chain" — applied to one physical part instance |
| **Generic** | `None` | "Service every chain" — applies to any part of the given type on a hook |

When creating a plan:
- `part = Some(id)` → `uid` is set to `None` (ownership enforced via part)
- `part = None` → `uid` is set to the current user (generic plans belong to users)

### Threshold Logic

Any threshold being `Some(value > 0)` activates that metric for comparison:

```typescript
// Frontend helper (frontend/src/lib/serviceplan.ts:56-65)
static valid(l: any) {
    return is_set(l.days) || is_set(l.hours) || is_set(l.km)
        || is_set(l.climb) || is_set(l.descend) || is_set(l.rides) || is_set(l.kJ);
}
```

A service plan can have multiple simultaneous thresholds. The frontend compares current accumulated usage (from the latest service + ongoing attachment) against these thresholds to determine if service is due.

### Immutables After Creation

Certain fields cannot be changed via `update()`:
- `part`, `what`, `hook`, `uid` — locked to prevent orphaning plan associations

---

## Database Schema

### `services` Table

```sql
CREATE TABLE services (
    id         uuid PRIMARY KEY           DEFAULT gen_random_uuid(),
    part_id    integer NOT NULL,          -- FK → parts(id)
    time       timestamptz NOT NULL,      -- service timestamp
    redone     timestamptz NOT NULL,      -- superseded time (MAX_TIME = valid)
    name       text NOT NULL,             -- service description
    notes      text NOT NULL DEFAULT '',  -- free-form notes
    usage      uuid NOT NULL,              -- FK → usages(id)
    successor  uuid,                       -- FK → services(id), predecessor chain
    plans      uuid[] DEFAULT ARRAY[],     -- FK → service_plans(id)
);

CREATE INDEX services_part_id_idx ON services(part_id);
CREATE INDEX services_time_idx ON services(time);
```

### `service_plans` Table

```sql
CREATE TABLE service_plans (
    id       uuid PRIMARY KEY,
    part     integer,                      -- FK → parts(id), nullable for generic plans
    what     integer NOT NULL,              -- PartTypeId (what type of part)
    hook     integer,                       -- Attachment hook for generic plans
    name     text NOT NULL,                 -- Plan description
    days     integer,                       -- Day-based threshold
    hours    integer,                       -- Hour-based threshold
    km       integer,                       -- Distance-based threshold
    climb    integer,                       -- Climbing threshold
    descend  integer,                       -- Descending threshold
    rides    integer,                       -- Activity count threshold
    uid      integer,                       -- User ID for generic plans
    energy   integer                        -- Energy threshold (kJ)
);

CREATE INDEX service_plans_part_idx ON service_plans(part) WHERE part IS NOT NULL;
CREATE INDEX service_plans_uid_idx ON service_plans(uid) WHERE uid IS NOT NULL;
```

---

## API Endpoints

### Service Endpoints (`axum/src/domain/service.rs`)

| Route | Method | Description |
|-------|--------|-------------|
| `/api/service/` | `POST` | Create a new service |
| `/api/service/` | `PUT` | Update an existing service |
| `/api/service/{id}` | `DELETE` | Delete a service |
| `/api/service/redo` | `POST` | Redo a service at a different time |

### ServicePlan Endpoints (`axum/src/domain/serviceplan.rs`)

| Route | Method | Description |
|-------|--------|-------------|
| `/api/plan/` | `POST` | Create a plan |
| `/api/plan/{id}` | `PUT` | Update a plan |
| `/api/plan/{id}/services` | `GET` | List services fulfilling this plan |

### Create Service Request (`NewService`, service.rs:43-50)

```rust
struct NewService {
    part_id:     PartId,          // which part was serviced
    time:        OffsetDateTime,  // when the service occurred
    name:        String,          // "Chain cleaning"
    notes:       String,          // optional details
    plans:       Vec<ServicePlanId>,  // associated service plans
}
```

---

## Frontend Model

### Service (`frontend/src/lib/service.ts:7-140`)

```typescript
class Service {
  id:          string | undefined;  // UUID
  part_id:     number;               // PartId
  time:        Date;                 // Service date
  redone:      Date;                 // Superseded date (deprecated, not actively used)
  name:        string;               // Service name
  notes:       string;               // Notes
  usage:       string;               // Usage UUID
  successor:   string | null;        // Successor service UUID
  plans:       string[];             // Associated plan UUIDs
  
  async create(part_id, time, name, notes, plans): Summary
  async update(): Summary
  async delete(): void
  async repeat(): Summary            // redo() — see frontend as "repeat"
  
  get_successor(s: Map<Service>): Service | null    // Follow successor chain
  history(depth, services): ...                              // Build service history tree
}
```

The `history()` method builds a tree of predecessor services for rendering in the UI, with depth-based indentation.

### ServicePlan (`frontend/src/lib/serviceplan.ts:77-338`)

```typescript
class ServicePlan extends Limits {
  id:       string | undefined;
  part:     number | null;           // Specific part or null for generic
  what:     number;                   // PartTypeId
  hook:     number | null;            // Hook for generic plans
  name:     string;                   // Plan description
  uid:      number | null;            // User for generic plans
  
  // Inherited from Limits:
  days, hours, km, climb, descend, rides, kJ   // threshold values
}

class Limits {
  static keys: ("days" | "rides" | "hours" | "km" | "climb" | "descend" | "kJ")[]
  static valid(l): boolean            // Check if any threshold is set
  
  to_object(): Record<string, number | null>
  set_from_object(a): void
}
```

### Integration with Attachment and Part data

ServicePlan checks current usage against thresholds using attachment state:

```typescript
// Uses these helpers to get current attached parts and their usage
import { att_at_hook, attachment_for_part, part_at_hook } from "./attachment";
import { services } from "./service";

// Frontend determines if a service is overdue by:
// 1. Getting the latest Service for the part
// 2. Computing: (current_time - service.time) against days threshold
// 3. Comparing accumulated usage in latest Service + ongoing attachment against km/hours/climb/etc.
```

---

## Data Flow: Full Cycle

```
1. User creates a ServicePlan:
   ┌──────────────────────────────────────────────────────┐
   │  ServicePlan: "Chain service"                        │
   │    part = Some(chain_id), hook = Bike               │
   │    km = 500, hours = 60, rides = 100                │
   └──────────────────────┬───────────────────────────────┘
                          │
2. User rides → Strava activities imported               ▼
   Attachments match activities to parts  ────▶ Usage accumulated

3. User logs a service (or reattaches the part):
   ┌──────────────────────────────────────────────────────┐
   │  Service::create(chain_id, time="Jul 1")             │
   │    → Activity::find(gear=5028, MIN_TIME, Jul 1)      │
   │    → Usage: {time: 25000, km: 850000, rides: 42}     │
   │    → stored in usages table                           │
   └──────────────────────┬───────────────────────────────┘
                          │
4. Frontend comparison:                                 ▼
   ┌──────────────────────────────────────────────────────┐
   │  Current usage (from latest Service + attachment)    │
   │  vs. ServicePlan thresholds:                         │
   │                                                      │
   │  km:      850,000 m / 500,000 m = 170% ← EXCEEDED   │
   │  hours:  25,000 s / 216,000 s = 11%                   │
   │  rides:     42 / 100 = 42%                            │
   │                                                      │
   │  → Show "Service Due" alert for chain                │
   └──────────────────────────────────────────────────────┘
                          │
5. User performs service → logs it:                       ▼
   ┌──────────────────────────────────────────────────────┐
   │  Service::create(chain_id, time="Aug 15")            │
   │    → Recalculates ALL services on chain after Jul 1  │
   │    → New service becomes latest                      │
   │    → Thresholds reset from new baseline              │
   └──────────────────────────────────────────────────────┘
```

---

## Key Design Patterns

1. **Usage is derived, not entered** — The `usage` field in both `Service` and `Attachment` is a foreign key to the `usages` table. The usage metrics are always computed by matching Strava activities against time windows (attachment periods or activity dates). Users never manually enter usage numbers.

2. **Successor chains** — Services link to their successors via `successor: Option<ServiceId>`. This creates a linked-list-like chain within the same part's service history. Deleting a non-leaf service rewires all its predecessors to point to `None`.

3. **Redo preserves history** — Unlike typical CRUD "update", the `redo()` method creates a new entry rather than modifying in place. This preserves the full history of "what I thought the maintenance status was and when". The old entry remains but its successor pointer changes.

4. **Plan ownership** — Specific plans (tied to a part) inherit the part's ownership. Generic plans (type-based) are owned by the user who created them. The `checkuser()` method enforces this distinction.

5. **Atomic usage propagation** — When attachments change, all affected services are recalculated in a single batch via `Usage::update_vec()`. This prevents partial updates where one service's usage is updated but others are stale.

---

## Tests

Both entities have full `#[cfg(test)]` suites using the in-memory `MemStore`:

- **`service.rs`** (test module starts at line 224): covers `Service::create`, `Service::update`, `Service::delete`, `Service::redo`, successor chain linking, usage calculation for main parts vs sub-parts, and recalculation on attachment changes.
- **`serviceplan.rs`** (test module starts at line 127): covers `ServicePlan::create` (specific vs generic mode), `ServicePlan::update` (immutability of `part`/`what`/`hook`/`uid`), `ServicePlan::delete`, threshold field round-trips, and ownership enforcement via `checkuser()`.

Run with: `SQLX_OFFLINE=true cargo test -p tb_domain`

---

## Key Files

| File | Role |
|------|------|
| `domain/src/entities/service.rs` | Service entity, CRUD, usage calculation, successor chain logic |
| `domain/src/entities/serviceplan.rs` | ServicePlan entity, threshold definitions, user/part ownership |
| `domain/src/traits/service.rs` | ServiceStore trait (get, create, update, delete, by_part) |
| `domain/src/traits/serviceplan.rs` | ServicePlanStore trait (get, create, update, delete, by_part/by_user) |
| `sqlx/src/store/service.rs` | PostgreSQL persistence for services |
| `sqlx/src/store/serviceplan.rs` | PostgreSQL persistence for service plans |
| `axum/src/domain/service.rs` | REST API handlers (create, update, delete, redo) |
| `axum/src/domain/serviceplan.rs` | REST API handlers (create, update, list services) |
| `frontend/src/lib/service.ts` | TypeScript Service class with CRUD and history traversal |
| `frontend/src/lib/serviceplan.ts` | TypeScript ServicePlan + Limits classes, threshold helpers |
| `sqlx/migrations/20250101000000_initial_schema.up.sql:119-165` | Database schema for services and service_plans tables |
