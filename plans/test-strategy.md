# TendaBike Test Strategy

## Overview

This document defines the test strategy for TendaBike, a bike maintenance tracker that syncs with Strava. The project currently has **no test framework configured** — CI only runs type checking (`npm run check`) and format validation (`cargo fmt --check`). This strategy outlines a phased approach to introducing tests across the backend (Rust) and frontend (Svelte/TypeScript).

---

## Current State

| Layer | Framework | CI Coverage | Notes |
|-------|-----------|-------------|-------|
| Backend `domain/` | None (`test = false`) | Unit test for `round_time()` only | Business logic crate — highest test priority |
| Backend `sqlx/` | None (`test = false`) | None | PostgreSQL store implementation |
| Backend `strava/` | None (`test = false`) | None | External API client |
| Backend `axum/` | None | None | HTTP web layer |
| Frontend | None | Type checking + format only | Svelte 5 components + stores |

---

## Testing Pyramid

```
                    /  E2E  \
                   / Integration \
                  /    Unit      \
                 -----------------
                | Domain Logic   |  ← Rust unit tests (fast, deterministic)
                | Frontend Utils |  ← TypeScript unit tests
                -----------------
               /   API / Store   \  ← Integration tests with test DB
              /   Strava Mocking  \  ← HTTP mocking for Strava API
             -----------------------
            |     Docker + CI      |  ← E2E smoke tests
           -------------------------
```

---

## Phase 1: Backend Domain Layer (High Priority)

### Goal: Test pure business logic in isolation — fast, no dependencies

#### 1.1 Unit Tests for Entity Methods

Test all entity methods that contain business rules. These are **fast, deterministic, and require no external dependencies**.

**Priority entities:**

| Entity | Test Areas |
|--------|-----------|
| [`Usage`](backend/domain/src/entities/usage.rs) | `Add` impl, `update()`, `delete_all()`, `get_vec()`, negation |
| [`Activity`](backend/domain/src/entities/activity.rs) | `register()`, `calculate_usage()`, `find()` — usage accumulation logic |
| [`Service`](backend/domain/src/entities/service.rs) | `create()`, `delete()` (predecessor chain handling), `redo()` |
| [`Part`](backend/domain/src/entities/part.rs) | `delete()` (conflict checks: attachments, services, plans), `checkuser()` |
| [`ServicePlan`](backend/domain/src/entities/serviceplan.rs) | `create()`, `update()` (immutable fields), `checkuser()` |
| [`Shop`](backend/domain/src/entities/shop.rs) | Ownership checks, subscription logic |
| [`Usage`](backend/domain/src/entities/usage.rs) | Arithmetic ops (`Add`, `Sub`, `Neg`), accumulation from activities |

#### 1.3 Attachment Entity — Critical Path Testing

**The Attachment entity is one of the most complex in the codebase.** It manages hierarchical part-to-gear relationships with timeline-based semantics. Test coverage for Attachment is critical because it directly drives usage accounting for all attached parts.

**Priority areas for Attachment tests:**

| Area | Source | Test Focus |
|------|--------|-----------|
| **Timeline logic** | [`Attachment`](backend/domain/src/entities/attachment.rs:40) | `attached`/`detached` time bounds, "still attached" (`detached == MAX_TIME`) semantics |
| **`attach_assembly()`** | [`attach_assembly()`](backend/domain/src/entities/attachment.rs:440) | Full assembly attach with subparts, successor/predecessor chain handling |
| **`detach_assembly()`** | [`detach_assembly()`](backend/domain/src/entities/attachment.rs:517) | Cascading detach of subparts, `all` flag behavior |
| **`attach_one()`** | [`attach_one()`](backend/domain/src/entities/attachment.rs:369) | Successor detection, predecessor merging, hook conflict resolution |
| **`shift_subparts()`** | [`shift_subparts()`](backend/domain/src/entities/attachment.rs:321) | Recursive subpart reassignment between gears |
| **`register_activity()`** | [`register_activity()`](backend/domain/src/entities/attachment.rs:258) | Usage accumulation across all parts in an assembly at a point in time |
| **`activities_by_part()`** | [`activities_by_part()`](backend/domain/src/entities/attachment.rs:225) | Activity filtering by attachment timeline windows |
| **`dispose_assembly()`** | [`dispose_assembly()`](backend/domain/src/entities/attachment.rs:534) | Disposal conflict detection for currently-attached parts |
| **`recover_assembly()`** | [`recover_assembly()`](backend/domain/src/entities/attachment.rs:584) | Restore from disposed state with subparts |
| **Validation** | [`attach_assembly()` lines 451-471](backend/domain/src/entities/attachment.rs:451) | Hook type validation (`parttype.hooks.contains()`), gear type compatibility |
| **`SumHash`** | Internal | Change tracking for batch updates across assembly operations |

**Example test structure:**

```rust
#[cfg(test)]
mod attachment_tests {
    use super::*;
    use crate::*;

    // Mock store implementations would go here

    #[tokio::test]
    async fn test_attach_simple_part() {
        // Arrange: create part, gear, valid hook
        let part = PartId(1);
        let gear = PartId(2);
        let hook = PartTypeId(10); // e.g., WHEEL hook on FRAME
        let time = datetime!(2024-01-15 10:00 UTC);

        // Act: attach part to gear at hook
        let result = attach_assembly(&mock_session(), part, time, gear, hook, false, &mut mock_store()).await;

        // Assert: verify attachment created with correct timeline
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.attachments.len(), 1);
        assert_eq!(summary.attachments[0].a.gear, gear);
        assert_eq!(summary.attachments[0].a.hook, hook);
    }

    #[tokio::test]
    async fn test_attach_conflicting_hook_replaces() {
        // Arrange: part A already attached to gear@hook
        // Act: attach part B to same gear@hook
        // Assert: A is detached at B's attach time, B is attached
    }

    #[tokio::test]
    async fn test_attach_invalid_hook_rejected() {
        // Arrange: part type does not support the target hook
        // Act: attempt attach
        // Assert: returns Error::BadRequest with hook compatibility message
    }

    #[tokio::test]
    async fn test_attach_with_subparts() {
        // Arrange: wheel assembly (rim + spoke + tire) attached to FRAME@WHEEL
        // Act: attach full assembly
        // Assert: all subparts attached with correct parent-child timeline
    }

    #[tokio::test]
    async fn test_detach_assembly_cascades() {
        // Arrange: assembly with subparts attached
        // Act: detach with all=true
        // Assert: all subparts detached, usages recalculated
    }

    #[tokio::test]
    async fn test_dispose_conflict_when_attached() {
        // Arrange: part has active attachment (detached > now)
        // Act: attempt dispose
        // Assert: returns Error::Conflict
    }

    #[test]
    fn test_attach_one_merges_adjacent() {
        // Arrange: existing attachment ends at time T
        // Act: create new attachment starting at T
        // Assert: merged into single attachment spanning both periods
    }

    #[tokio::test]
    async fn test_register_activity_accumulates_usage() {
        // Arrange: gear with multiple attachments, activity at time T
        // Act: register activity
        // Assert: all affected part usages updated, activity usage distributed
    }
}
```

**Key invariant tests:**

```rust
// Invariant: total usage across assembly parts == total activity usage
#[tokio::test]
async fn test_usage_conservation_during_attach() {
    // After attach_assembly(), sum of all part usages should equal
    // the sum of activities between attached and detached times
}

// Invariant: no two attachments for same part overlap
#[tokio::test]
async fn test_no_overlapping_attachments() {
    // After any sequence of attach/detach operations,
    // verify no two attachments for the same part_id overlap in time
}

// Invariant: hook exclusivity — one part per gear@hook at any time
#[tokio::test]
async fn test_hook_exclusivity() {
    // After any attach operation, verify at most one part per (gear, hook) at any time T
}
```

**Example test structure:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_usage_addition() {
        let u1 = Usage { time: 3600, distance: 50000, climb: 500, descend: 500, energy: Some(1000), count: 1, ..Default::default() };
        let u2 = Usage { time: 1800, distance: 25000, climb: 250, descend: 250, energy: Some(500), count: 1, ..Default::default() };
        let result = &u1 + &u2;
        assert_eq!(result.time, 5400);
        assert_eq!(result.distance, 75000);
        assert_eq!(result.count, 2);
    }

    #[test]
    fn test_usage_negation() {
        let u = Usage { time: 3600, distance: 50000, energy: Some(1000), ..Default::default() };
        let negated = -&u;
        assert_eq!(negated.time, -3600);
        assert_eq!(negated.distance, -50000);
    }
}
```

#### 1.2 Module-Level Tests

Test utility functions like [`round_time()`](backend/domain/src/lib.rs:40):

```rust
#[test]
fn test_round_time_edge_cases() {
    // Test leap years, timezone offsets, DST boundaries
}
```

#### 1.3 Enable Testing in Cargo.toml

Remove `test = false` from the following crates:
- [`backend/domain/Cargo.toml`](backend/domain/Cargo.toml)
- [`backend/sqlx/Cargo.toml`](backend/sqlx/Cargo.toml)
- [`backend/strava/Cargo.toml`](backend/strava/Cargo.toml)

---

## Phase 2: Backend Integration Layer

### Goal: Test store implementations against a real PostgreSQL instance

#### 2.1 Test Database Setup

Use a dedicated test database (`tendabike_test`) with automatic migration:

```rust
// In sqlx crate
use sqlx::postgres::PgPoolOptions;

async fn test_pool() -> PgPool {
    let url = std::env::var("TEST_DB_URL")
        .unwrap_or_else(|_| "postgres://localhost/tendabike_test".into());
    
    let pool = PgPoolOptions::new().max_connections(5).connect(&url).await.unwrap();
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    
    pool
}
```

#### 2.2 Integration Test Structure

Create `backend/sqlx/tests/` with integration tests:

| Test Module | Focus |
|-------------|-------|
| `activity.rs` | CRUD, user ownership, gear association |
| `part.rs` | CRUD, deletion conflict checks, shop delegation |
| **`attachment.rs`** | **CRUD, timeline queries, assembly operations, usage recalculation** |
| `service.rs` | Creation, history chain, usage recalculation |
| `serviceplan.rs` | CRUD, plan matching against services |
| `usage.rs` | Upsert behavior, retrieval by ID |
| `attachment.rs` | Part attachment lifecycle |
| `user.rs` | CRUD, onboarding status transitions |
| `shop.rs` | Shop management, subscription flows |
| `transactions.rs` | `begin()` / `commit()` semantics, rollback on error |

**Attachment-specific integration tests:**

| Test | SQL Query Focus |
|------|-----------------|
| `attachment_create_inserts_correctly` | Verify `INSERT INTO attachments` stores all fields, returns correct UUID |
| `attachment_query_by_gear_at_time` | Test `WHERE gear = $1 AND attached <= $2 AND detached > $2` — find active attachments |
| `attachment_query_by_part_at_time` | Test `WHERE part_id = $1 AND attached <= $2 AND detached > $2 FOR UPDATE` — row-level locking |
| `attachment_query_successor_detection` | Test `attachment_find_successor()` — detecting next part on same hook |
| `attachment_merge_adjacent` | Create adjacent attachments → verify they merge into single record |
| `assembly_attach_creates_usage_records` | Verify `attach_assembly()` creates corresponding Usage records |
| `assembly_detach_recalculates_service_usages` | Verify `Service::recalculate()` is called correctly after detach |
| `activity_registration_updates_all_attachments` | Register activity on gear → verify all attachment usages updated |
| `attachment_dispose_prevents_active_disposal` | Active attachment → verify dispose returns Conflict |

#### 2.3 Transaction Testing

Critical to verify the **commit-is-required** pattern documented in [`backend/AGENTS.md`](backend/AGENTS.md:25):

```rust
#[tokio::test]
async fn test_transaction_commit() {
    let pool = test_pool().await;
    let mut store = pool.begin().await.unwrap();
    
    // Perform operations
    store.part_delete(PartId(1)).await.unwrap();
    
    // Commit is required — verify data is visible after commit
    store.commit().await.unwrap();
    
    // Verify persisted
    let result = store.partid_get_part(PartId(1)).await;
    assert!(result.is_err()); // was deleted
}

#[tokio::test]
async fn test_transaction_rollback() {
    let pool = test_pool().await;
    let store = pool.begin().await.unwrap();
    
    // Intentionally do NOT commit — verify data is NOT visible
    // (This test validates transaction isolation)
}
```

---

## Phase 3: Backend Web Layer (axum)

### Goal: Test API endpoints with mocked store and session

#### 3.1 HTTP Endpoint Tests

Use `axum::testing::TestClient` for each resource:

| Endpoint Group | Test Cases |
|---------------|-----------|
| `GET /api/user/` | Auth flow, session validation |
| `POST /api/part/` | Validation, ownership assignment |
| `DELETE /api/part/{id}` | Conflict detection (attached parts, active plans) |
| `POST /api/attachment/` | Attach part to gear@hook, hook conflict handling |
| `POST /api/attachment/detach/` | Cascade detach, subpart handling (`all` flag) |
| `POST /api/attachment/dispose/` | Active attachment conflict detection |
| `POST /api/service/` | Usage calculation, chain linking |
| `DELETE /api/service/{id}` | Predecessor chain update |
| `GET /api/activ/*` | Ownership enforcement |

#### 3.2 Mock Store Trait

Create a mock implementation of `Store` trait for unit testing handlers:

```rust
struct MockStore {
    parts: Arc<Mutex<HashMap<PartId, Part>>>,
    activities: Arc<Mutex<HashMap<ActivityId, Activity>>>,
    // ... other entities
}

#[async_trait]
impl PartStore for MockStore {
    async fn partid_get_part(&mut self, id: PartId) -> TbResult<Part> {
        self.parts.lock().await.get(&id).cloned().ok_or(Error::NotFound(...))
    }
    // ... implement all trait methods
}
```

#### 3.3 Session Mock

```rust
struct MockSession {
    user_id: UserId,
    shop: Option<ShopId>,
    admin: bool,
}

impl Session for MockSession {
    fn user_id(&self) -> UserId { self.user_id }
    fn shop(&self) -> Option<ShopId> { self.shop }
    fn is_admin(&self) -> bool { self.admin }
    fn set_shop(&mut self, shop: Option<ShopId>) -> TbResult<()> {
        self.shop = shop; Ok(())
    }
}
```

#### 3.4 Error Mapping Tests

Verify [`AppError`](backend/axum/src/error.rs:24) maps correctly to HTTP status codes:

```rust
#[test]
fn test_error_to_http_status() {
    assert_eq!(status_for(Error::NotAuth("".into())), 401);
    assert_eq!(status_for(Error::Forbidden("".into())), 403);
    assert_eq!(status_for(Error::NotFound("".into())), 404);
    assert_eq!(status_for(Error::BadRequest("".into())), 400);
    assert_eq!(status_for(Error::Conflict("".into())), 409);
    assert_eq!(status_for(Error::TryAgain("".into())), 429);
    assert_eq!(status_for(Error::DatabaseFailure(...)), 500);
}
```

---

## Phase 4: Strava Integration Layer

### Goal: Test external API client with HTTP mocking

#### 4.1 HTTP Mocking with `wiremock`

Mock the Strava API endpoints:

| Endpoint | Test |
|----------|------|
| `GET /oauth/token` | Token exchange, refresh |
| `GET /oauth/authorization` | Authorization redirect URL |
| `GET /authenticated` | User profile fetching |
| `GET /activities` | Pagination, rate limiting, error handling |
| `POST /activities` | Activity creation (Garmin uploads) |
| `GET /gear/{id}` | Gear data fetching |

#### 4.2 StravaActivity Conversion Tests

Test [`StravaActivity::into_activity()`](backend/strava/src/activity.rs:59) conversion logic:

```rust
#[test]
fn test_strava_activity_conversion() {
    let strava = StravaActivity {
        id: 12345,
        type_: "Ride".into(),
        name: "Morning ride".into(),
        start_date: /* ... */,
        // ...
    };
    // Verify conversion to domain Activity
}

#[test]
fn test_strava_activity_type_mapping() {
    // Test all type mappings: Ride, Run, Hike, etc. → ActTypeId
}

#[test]
fn test_strava_activity_timezone_conversion() {
    // Test UTC offset handling
}
```

#### 4.3 Webhook Event Processing

Test [`Event`](backend/strava/src/event.rs) handling:

```rust
#[test]
fn test_webhook_verification() {
    // Verify webhook signature validation
}

#[test]
fn test_event_subscription_creation() {
    // Test Strava subscription lifecycle
}
```

---

## Phase 5: Frontend Testing

### Goal: Test Svelte components and utility functions

#### 5.1 Utility Function Tests

Test functions in [`frontend/src/lib/store.ts`](frontend/src/lib/store.ts):

| Function | Tests |
|----------|-------|
| [`roundTime()`](frontend/src/lib/store.ts:8) | Quarter-hour rounding edge cases |
| [`get_days()`](frontend/src/lib/store.ts:15) | Negative ranges, same-day |
| [`fmtDate()`](frontend/src/lib/store.ts:19) | `undefined` input, locale formatting |
| [`fmtRange()`](frontend/src/lib/store.ts:23) | Open-ended ranges, `maxDate` handling |
| [`fmtSeconds()`](frontend/src/lib/store.ts:29) | Negative values, zero, single-digit minutes |
| [`fmtNumber()`](frontend/src/lib/store.ts:39) | `undefined`, locale-aware formatting |
| [`myfetch()`](frontend/src/lib/store.ts:43) | 204 NO_CONTENT → null, 401 redirect |
| [`checkStatus()`](frontend/src/lib/store.ts:58) | Error response text extraction |

**Attachment utility function tests** — [`frontend/src/lib/attachment.ts`](frontend/src/lib/attachment.ts):

| Function | Tests |
|----------|-------|
| [`Attachment.isAttached()`](frontend/src/lib/attachment.ts:32) | Time within `[attached, detached)`, edge at boundaries, `MAX_TIME` for still-attached |
| [`Attachment.isDetached()`](frontend/src/lib/attachment.ts:39) | `detached < maxDate` detection |
| [`Attachment.isEmpty()`](frontend/src/lib/attachment.ts:43) | `attached >= detached` detection |
| [`Attachment.activities()`](frontend/src/lib/attachment.ts:47) | Filter activities by gear + timeline |
| [`att_at_hook()`](frontend/src/lib/attachment.ts:56) | Find currently-attached part at specific gear/hook |
| [`part_at_hook()`](frontend/src/lib/attachment.ts:74) | Return part_id or fallback to gear |
| [`attachment_for_part()`](frontend/src/lib/attachment.ts:87) | Find attachment at specific point in time |
| [`attachees_for_gear()`](frontend/src/lib/attachment.ts:98) | List all parts attached to a gear right now |

**Recommended framework: [Vitest](https://vitest.dev/) with [jsdom](https://github.com/jsdom/jsdom)**

Vitest integrates directly with Vite (no config migration needed) and supports Svelte component testing.

#### 5.2 Store Pattern Tests

Test [`mapable()`](frontend/src/lib/mapable.ts:20) behavior:

```typescript
import { get } from 'svelte/store';
import { mapable } from '../lib/mapable';

describe('mapable', () => {
  it('sets map from array', () => {
    const store = mapable('id');
    store.setMap([{ id: 1, name: 'A' }, { id: 2, name: 'B' }]);
    const map = get(store);
    expect(map['1']).toEqual({ id: 1, name: 'A' });
    expect(map['2']).toEqual({ id: 2, name: 'B' });
  });

  it('updates map incrementally', () => {
    const store = mapable('id');
    store.setMap([{ id: 1, name: 'A' }]);
    store.updateMap([{ id: 1, name: 'Updated' }]);
    const map = get(store);
    expect(map['1'].name).toBe('Updated');
  });

  it('deletes items from map', () => {
    const store = mapable('id');
    store.setMap([{ id: 1, name: 'A' }]);
    store.deleteItem('1');
    const map = get(store);
    expect(map).toEqual({});
  });
});
```

#### 5.3 Component Tests

Use [`@testing-library/svelte`](https://testing-library.com/docs/svelte-testing-library/intro/) for component testing:

| Component | Tests |
|-----------|-------|
| [`Widgets/TypeForm.svelte`](frontend/src/Widgets/TypeForm.svelte) | Validation, submit handling |
| [`Service/ServiceModal.svelte`](frontend/src/Service/ServiceModal.svelte) | Form lifecycle, success/error states |
| [`Widgets/SelectPart.svelte`](frontend/src/Widgets/SelectPart.svelte) | Part filtering, selection |
| [`Activity/ActName.svelte`](frontend/src/Activity/ActName.svelte) | Rendering, edit mode |
| [`Widgets/DateTime.svelte`](frontend/src/Widgets/DateTime.svelte) | Date/time formatting |
| [`Widgets/UsageChips.svelte`](frontend/src/Usage/UsageChips.svelte) | Usage display formatting |

---

## Phase 6: End-to-End Smoke Tests

### Goal: Verify the full stack works in Docker

#### 6.1 Docker Compose Test Environment

```yaml
# docker-compose.test.yml
version: '3.8'
services:
  test-db:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: tendabike_test
      POSTGRES_HOST_AUTH_METHOD: trust
    
  tendabike:
    build: .
    depends_on: [test-db]
    environment:
      DB_URL: postgres://test-db/tendabike_test
      BIND_ADDR: 0.0.0.0:8000
```

#### 6.2 E2E Test Scenarios

| Scenario | Steps |
|----------|-------|
| Fresh startup | Start containers → verify migrations run → health check passes |
| Strava OAuth flow | Initiate OAuth → callback → session created → user data fetched |
| Part lifecycle | Create part → attach gear → log activity → verify usage update |
| Service scheduling | Create service plan → log service → verify plan fulfillment |
| Shop delegation | Create shop → assign to part → verify access control |

---

## CI Integration

### Updated GitHub Actions Workflow

```yaml
# .github/workflows/test.yml (updated)
name: Test
on:
  pull_request:
    branches: ["main"]
  workflow_dispatch:

env:
  CARGO_TERM_COLOR: always
  SQLX_OFFLINE: true

jobs:
  frontend:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: ./frontend
    steps:
      - uses: actions/checkout@v6
      - uses: actions/setup-node@v6
        with:
          cache: npm
          cache-dependency-path: frontend/package-lock.json
      - run: npm install
      - run: npx prettier --check .
      - run: npm run check:ci
      - run: npm run test  # NEW: unit tests with Vitest

  rust-unit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo test --release  # Now works with test = true

  rust-integration:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_DB: tendabike_test
          POSTGRES_HOST_AUTH_METHOD: trust
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo install cargo-deny  # Optional: dependency checking
      - env:
          TEST_DB_URL: postgres://localhost/tendabike_test
        run: cargo test --test '*' --release

  docker:
    # Existing workflow unchanged
```

---

## Testing Tools Recommendation

| Layer | Tool | Purpose |
|-------|------|---------|
| Rust unit | `cargo test` (built-in) | Fast unit tests, doctests |
| Rust integration | `tokio-test` | Async test utilities |
| Rust HTTP mocking | `wiremock` | Mock Strava API |
| Rust test DB | `testcontainers-rs` or manual | PostgreSQL test instances |
| Frontend unit | `Vitest` | Fast TypeScript testing |
| Frontend component | `@testing-library/svelte` | Component rendering tests |
| Frontend mocking | `msw` (Mock Service Worker) | API response mocking |

---

## Test Coverage Goals

| Phase | Target Coverage | Rationale |
|-------|-----------------|-----------|
| 1. Domain unit | 80%+ | Business logic is core — must be reliable |
| 2. SQLx integration | 70%+ | Data layer correctness critical |
| 3. Axum API | 60%+ | HTTP contracts, error mapping |
| 4. Strava layer | 50%+ | External API — focus on conversion logic |
| 5. Frontend utilities | 70%+ | Pure functions are easy to test |
| 6. Frontend components | 40%+ | Svelte components harder to isolate |

---

## Migration Plan

### Step 1: Enable Testing Infrastructure (Week 1-2)

1. Remove `test = false` from `domain/`, `sqlx/`, `strava/` Cargo.toml files
2. Add `tokio` with `test-util` feature to `dev-dependencies` in relevant crates
3. Add `vitest` + `@testing-library/svelte` to frontend `devDependencies`
4. Update CI workflow to run new tests

### Step 2: Domain Layer Tests (Week 2-3)

1. Write tests for `Usage` arithmetic operations
2. Write tests for `Activity` usage accumulation
3. Write tests for `Part` deletion conflict detection
4. Write tests for `Service` chain management
5. Write tests for `round_time()` edge cases

### Step 3: Integration Tests (Week 3-4)

1. Set up test database provisioning
2. Write CRUD integration tests for each entity
3. Test transaction semantics
4. Test ownership enforcement across boundaries

### Step 4: API & Strava Tests (Week 4-5)

1. Create mock store implementation
2. Write endpoint tests for critical paths
3. Set up `wiremock` for Strava API
4. Test StravaActivity conversion

### Step 5: Frontend Tests (Week 5-6)

1. Configure Vitest for Svelte
2. Write utility function tests
3. Write mapable() store tests
4. Write critical component tests

### Step 6: E2E Smoke Tests (Week 6-7)

1. Create docker-compose test environment
2. Write smoke tests for full lifecycle scenarios
3. Add to CI pipeline

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Test DB availability in CI | Use PostgreSQL service container in GitHub Actions |
| Strava API rate limits during tests | Use `wiremock` — no real API calls in CI |
| Svelte 5 migration conflicts | Start with utility tests first (no Svelte deps) |
| Test flakiness from timing | Use deterministic time in tests where possible |
| Test database cleanup | Use transactions that rollback, or unique test DB per run |

---

## Existing Test Reference

The project already has one passing unit test in [`backend/domain/src/lib.rs:57-75`](backend/domain/src/lib.rs:57):

```rust
#[test]
fn test_round_time() {
    assert_eq!(
        round_time(datetime!(2020-01-01 0:00:00.0000 UTC)),
        datetime!(2020-01-01 0:00 UTC)
    );
    // ... 4 test cases total
}
```

This demonstrates the existing testing pattern — pure function tests with `#[test]` attribute and `time::macros::datetime!` for time literals. New tests should follow this same style.
