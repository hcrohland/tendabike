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

## Due-ness ownership

The due-ness **resolution** — remaining-threshold math, the 5% warn band and severity ladder, the specific-beats-generic plan ladder, covered-part exclusion, and next-due — is a **client-side derivative** owned by `frontend/src/lib/serviceplan.ts`. It is computed from the entity state the client already holds (plans, parts, services, usages, attachments) against the clock. Derivatives re-evaluate on store writes — hydration, the 60-second drain, write merges — not on clock passage; a midnight band flip waits for the next data change or re-hydration (accepted staleness policy, spec #323). This is the domain-flow pattern (`docs/agents/domain-flow.md`): the domain computes entity state; the client combines it to compute derivatives. Due-ness is a derivative, so it lives where the clock and the merged state live — the client. Recorded in ADR-0002 (`docs/adr/0002-plan-due-ness-is-a-client-side-derivative.md`), which supersedes the backend-placement ADR-0001.

**Backend boundary:** the backend provides CRUD for `Service` and `ServicePlan` plus the `unlink_plan` unlink (`Service::unlink_plan`, called from `ServicePlan::delete`; owner = the part's owner for a specific plan, the plan's `uid` for a generic one) — deleting a plan removes its id from the owner's services so no service references a deleted plan. It does **not** compute or store due-ness: there is no due-ness module, no `plan_status` field on the `Summary`, and no time-parameterized endpoint.

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

### How Plans Are Scoped & Created

A plan's scope is set by three fields — `part`, `what`, `hook`. Together they select which physical part(s) the plan applies to.

| Field | Value | Meaning |
|-------|-------|---------|
| `part` | `null` | **Generic** — a template applying to every gear in the category |
| `part` | a part id | **Specific** — tied to one gear or one component |
| `what` | a `PartTypeId` | The part type the plan concerns (the component type for generic plans; the part's own type for specific ones) |
| `hook` | `null` | The gear/body itself, or a component-specific plan |
| `hook` | a `PartTypeId` | The attachment point the plan applies to |

#### Three scopes (in precedence order)

The resolver `plans_for_attachee` (`frontend/src/lib/serviceplan.ts:242`, private interior, reached through the `plansForAssembly` / `plansForPart` doors) picks the *most specific* plan that covers a component:

1. **Component-specific** — `part = <component_id>`, `hook = null`. "Service this exact tire."
2. **Gear-specific** — `part = <gear_id>`, `what = <type>`, `hook = <hook>`. "Service this bike's front tire."
3. **Generic** — `part = null`, `what = <type>`, `hook = <hook>`. "Service every bike's front tire."

Scopes 1 and 2 **override** scope 3: if a component already has a specific plan, the generic plan no longer applies to it. The frontend de-duplicates the overall list on this rule — the covered-part exclusion in `gears_of_plan` (`serviceplan.ts:164`, private interior; door `gearsForPlan`) skips any gear whose component at the hook is covered by a gear-level *or* component-level specific plan.

> **Body (whole-bike) case:** `hook = null`. A generic body plan is `part = null, what = <bike type>, hook = null` ("service every bike"); a specific one is `part = <bike_id>, hook = null`.

#### Ownership

- Specific plan (`part = Some(id)`) → `uid = None`; ownership is enforced through the part (`checkuser`).
- Generic plan (`part = null`) → `uid = <current user>`; the user owns their templates.

#### Creation flows

All plans are created from the **New Plan** modal (`frontend/src/ServicePlan/NewPlan.svelte:16`), which behaves differently per page:

| Entry point | `part` | `what` / `hook` | Scope |
|-------------|--------|-----------------|-------|
| Component page (a part that isn't a gear) | `= component.id` | `= component.what`, `hook = null` (locked) | Component-specific |
| Gear page → pick subtype+hook, gear = **a bike** | `= bike.id` | subtype + hook | Gear-specific |
| Gear page → pick subtype+hook, gear = **any** | `= null` | subtype + hook | Generic |
| Gear page → pick **body**, gear = a bike | `= bike.id` | bike type, `hook = null` | Whole-bike (specific) |
| Gear page → pick **body**, gear = **any** | `= null` | bike type, `hook = null` | Whole-bike (generic) |

- On a **component page** the modal is locked (`no_gear = true`): only name + limits are editable; `part`/`what`/`hook` are fixed to that component.
- On a **gear page** the modal shows a `TypeForm` (body, or a subtype+hook) and a `GearForm` (a specific bike, or **any** → `part = null`).

Once created, `part`/`what`/`hook`/`uid` are **immutable** — `update()` refuses to change them (`serviceplan.rs:104-108`), so a plan's scope can only change by deleting and recreating it.

#### Resolution (which plans show for a part)

- **Single-gear view** (`PlanList` with a gear): the `plansForAssembly` door (`serviceplan.ts:410`) returns the part's own plans plus the plans of the parts it assembles through its type's subtype hooks, resolved through `plans_for_attachee` — specific plans where they exist, generic ones only as a fallback — unsorted; `PlanList` sorts with `planCmp`.
- **Overall list** (`PlanList` without a gear): shows every plan in the category; each `PlanBlock` renders `gearsForPlan(plan, $parts, $attachments, $plans)` — the gears the plan currently applies to, with specific-covered components excluded (see the override note above).

### Threshold Logic

Any threshold being `Some(value > 0)` activates that metric for comparison:

```typescript
// Frontend helper (frontend/src/lib/serviceplan.ts:56-65)
static valid(l: any) {
    return is_set(l.days) || is_set(l.hours) || is_set(l.km)
        || is_set(l.climb) || is_set(l.descend) || is_set(l.rides) || is_set(l.kJ);
}
```

A service plan can have multiple simultaneous thresholds. This comparison is the **due-ness resolution** — a client-side derivative the frontend owns (see [Due-ness ownership](#due-ness-ownership)); it compares current accumulated usage (from the latest service + ongoing attachment) against these thresholds to determine if service is due.

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
| `/api/plan/{id}` | `DELETE` | Delete a plan; response is the owner's services with the plan id unlinked |

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

### ServicePlan (`frontend/src/lib/serviceplan.ts:77`)

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

The due-ness rule itself lives in the same module behind eight narrow doors (plus the exported `planCmp` comparator); doors take the store *values* their call site reads, so each call site stays reactive only to the stores it reads. The doors return the unsorted walk; sorting is the caller's job via `planCmp` (type → hook → part → id):

| Door | Maps read | Answers |
|------|-----------|---------|
| `plansForPart` | plans, attachments | plans for one part at a pinned time or now (the one time-parameterized door) |
| `plansForAssembly` | plans, attachments | the part's own plans plus its assembled subtypes, resolved |
| `duesForPlans` | services, usages | per-limit remaining and the nearest next-due for a part |
| `alertCounts` | parts, services, usages, attachments | warn/alert band counts over a set of plans |
| `gearsForPlan` | parts, attachments, plans | the gears a plan applies to (covered parts excluded) |
| `partForPlanGear` | parts, attachments | the component a plan's gear row points at |
| `isTemplate` | plans | whether the plan is still an unbound (generic) template |
| `localizeLimitKey` | — | localized label for a limit key |

Everything else — the resolution ladder (`plans_for_this_part` / `plans_for_attachee` / `plans_at_hook` / `plans_for_subtype`), the `due_for` / `alert_for` folds, `part_for_plan`, `gears_of_plan` — is private module interior.

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

- **`service.rs`** (test module starts at line 235): covers `Service::create`, `Service::update`, `Service::delete`, `Service::redo`, successor chain linking, usage calculation for main parts vs sub-parts, and recalculation on attachment changes.
- **`serviceplan.rs`** (test module starts at line 133): covers `ServicePlan::create` (specific vs generic mode), `ServicePlan::update` (immutability of `part`/`what`/`hook`/`uid`), `ServicePlan::delete` including the `unlink_plan` unlink — owner scoping and the shop-session part-owner case (SP-12 / SP-12a / SP-12b) — threshold field round-trips, and ownership enforcement via `checkuser()`.

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
| `axum/src/domain/serviceplan.rs` | REST API handlers (create, update, delete + unlink) |
| `frontend/src/lib/service.ts` | TypeScript Service class with CRUD and history traversal |
| `frontend/src/lib/serviceplan.ts` | TypeScript ServicePlan + Limits classes, the due-ness rule (eight doors, `planCmp`), threshold helpers |
| `sqlx/migrations/20250101000000_initial_schema.up.sql:119-165` | Database schema for services and service_plans tables |
