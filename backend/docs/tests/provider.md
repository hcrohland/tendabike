# Provider tests

Shared conventions (the seam model, writing conventions, the mockall `.times` gotcha, the shared provider kit) are in [tests.md](../tests.md).

## Modules

| Module | Responsibility |
|--------|---------------|
| `dispatch.rs` | Provider-agnostic dispatch: `sync_user` (per-user onboarding), `sync_users` (admin re-trigger), `import_gear_user`, `queue_sync_user` (two-step split) |
| `facade.rs` | `EntityFacade` trait — the entity-op door (gear CRUD, user ops); `MockEntityFacade` via `#[automock]` |
| `queue.rs` | Provider-agnostic event-queue kernel (spec #1029): `get_event` (the read-side rules — rate-limit gate, latest-per-key dedup) and `fail_event` (the four-arm failure policy) — written once, generic over the `ProviderSpec` marker, driven through the single `Store` interface (the event identity is the marker's type-level accessors) |
| `strava/` | Strava driver: `user` (onboard, gear, stats), `event` (sync enqueue, processing — one event per `process` call), `gear`, `activity`, `session` (token cache) |
| `icu/` | intervals.icu driver: `user` (onboard, gear, stats), `event` (sync enqueue, processing — one event per `process` call, one `SYNC` batch per call), `gear`, `activity`, `http` |
| `test_support.rs` | Shared provider kit: canned-response kit (`queue`, `queue_error`, `requests`, `begin_test`) |

## Shared pattern

Each driver owns its **provider → domain mapping** (JSON → domain entities) and the driver-logic free functions over it. The logic runs for real against the driver's `MockStore<Spec>` (the `#[automock]` of the single `Store` trait); domain ops go through `MockEntityFacade` (`C` inferred as the mock store); the HTTP API is stubbed behind the shared canned-response kit. Assert on canned store outcomes for state; on `requests()` for API calls.

## Strava driver

- **`MockStore<StravaSpec>`** (`store.rs`) — the `Store` automock.
- **`strava_session` / `strava_admin_session`** (`strava/test_support.rs`) — build a `MockSession` with a warm `TokenCache` (no refresh). `strava_session` begins the thread-local canned-response scope. Extra: `deauthorizes() -> Vec<StravaId>` records `deauthorize()` calls.
- **JSON helpers**: `activity_json(id, type, gear)` (timestamp `1767348000`, distance `25000.0`, elevation `300.0`), `gear_json(id, frame_type)` (`None` = shoes), `strava_user(tbid, stravaid, enabled)`.

```rust
use crate::strava::test_support::{setup, gear_json};
use crate::test_support::queue;

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let (mut store, _facade, _session) = setup();
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // ...
    Ok(())
}
```

- The `CANNED_STRAVA` global (`canned_strava(uri, json)`, feature `test-support`) is a separate, process-wide seam for the `tb_sqlx` executor-seam suite (whose session runs on a thread the thread-local map never reaches). The thin free functions consult the thread-local map first, then `CANNED_STRAVA`.

**Gotchas:** `refresh_token().map(|t| t.secret())` borrows a local — use `.map(|t| t.secret().to_string())`. Test fn names must not shadow module functions under test. `42.into().method(...)` is ambiguous — use `StravaId::from(42)`.

## ICU driver

- **`MockStore<IcuSpec>`** (`store.rs`) — the `Store` automock.
- **`setup()`** (`icu/test_support.rs`) — builds a fresh `MockStore<IcuSpec>` + `MockEntityFacade` and calls `begin_test()`. No token cache to seed (icu tokens are durable, read from `icu_users`), so `setup()` is the only entry point.
- **Fixtures**: `icu_user(tbid, icuathlete, enabled)`, `user(id)`, `icu_id(n)`, `part(id, owner, what, source)`, `activity_json(id, type, gear)` (timestamp `1767348000`, distance `25000.0`, moving time `3300`, joules `4000.0`), `stub_activity_json(id, type)` (distance + moving_time null), `gear_json(id, type)`.

```rust
use crate::icu::test_support::{setup, activity_json};
use crate::test_support::queue;

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let (mut store, _facade) = setup();
    queue("/activity/abc123", &activity_json("abc123", "Ride", Some("g1")));
    // ...
    Ok(())
}
```

- `icu_user(…, enabled)` sets `access_token` to `Some("test-token")` when enabled, `None` otherwise — read by `http::request_json` to authenticate.

### Test files

| File | Covers |
|------|--------|
| `queue/queue_tests.rs` | The kit's event-queue kernel (spec #1029): `get_event` (the read-side rules — the rate-limit gate, the latest-per-key dedup, the drop-of-older) and `fail_event` (the four-arm failure policy), driven by a single `MockStore` over a test `ProviderSpec` marker |
| `activity/activity_tests.rs` | Activity import: JSON → `Activity` mapping, type mapping, stub detection, gear reference |
| `gear/gear_tests.rs` | Gear import: JSON → `Part` mapping, `into_partid` upsert |
| `event/event_tests.rs` | Event queue: `process` (one event per call), `insert_sync`, `process_sync` (one batch per call), `accept` |
| `user/user_tests.rs` | User ops: `upsert`, `get_all_stats`, `onboard`, token management |
| `http/http_tests.rs` | HTTP seam: `request_json` against canned responses |
