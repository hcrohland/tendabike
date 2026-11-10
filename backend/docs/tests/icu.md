# ICU tests

All tests run in-memory (no database, no network); one `<name>_tests.rs` module per module under `provider/src/icu/<name>/`, declared `#[cfg(test)]` in `activity.rs`, `event.rs`, `gear.rs`, `user.rs`, `backfill.rs`, `http.rs`.

## Behavior ownership

The ICU module (`tb_provider::icu`) owns the **intervals.icu → domain mapping**: translating icu JSON into domain entities (gear JSON → `Part`, activity JSON → `Activity`, user JSON → `IcuUser`) and the icu-logic free functions over it (activity import, gear hydration, event processing, backfill).

- **What it mocks** — the **IcuDataStore** (the icu-specific I/O boundary): `MockIcuDataStore` is a `mockall` `#[automock]` of the `IcuDataStore` trait, so the icu-logic runs for real against canned store outcomes. Domain operations are mocked via `MockDomainFacade` (the `DomainFacade` seam). The **intervals.icu HTTP API** is stubbed behind the thread-local canned-response map (`queue` / `queue_error` / `requests`).
- **What it delegates to the seam** — the **store I/O** (real Postgres) lives in the sqlx seam suites (see [`sqlx.md`](sqlx.md)); the **frame-push / orchestration** (deciding what to push after hydration) lives in `tb_exec` (see [`exec.md`](exec.md)); the **domain-level results** live in the `tb_domain` suite (see [`domain.md`](domain.md)).

## Key infrastructure

- **`MockIcuDataStore`** (`provider/src/icu/traits.rs`, via `#[automock]`) — the `IcuDataStore` automock: icu-table I/O is canned, domain ops go through `MockDomainFacade` (`C` inferred as `MockIcuDataStore`). The icu-logic tests (activity import, gear hydration, event processing, backfill) run the logic for real and assert on canned outcomes.
- **`setup()`** (`provider/src/icu/test_support.rs`) — builds a fresh `MockIcuDataStore` and a `MockDomainFacade` (with `C` = `MockIcuDataStore`), and calls `begin_test()` to mark the thread in test mode so the HTTP seam's canned responses are replayed. Unlike the Strava driver there is no token cache to seed (intervals.icu tokens are durable, read straight from `icu_users`), so `setup()` is the only entry point.
- **HTTP seam** (`provider/src/icu/test_support/seam.rs`) — thread-local canned responses for the intervals.icu API. The canned-response surface is free functions:
  - `begin_test()` — mark the thread in test mode and clear any canned responses/records (per-test isolation)
  - `queue(uri, json)` — script a JSON response body for an **exact** request URI (FIFO per URI)
  - `queue_error(uri, error)` — script an error for an exact URI; unscripted URIs yield `Error::BadRequest`
  - `requests() -> Vec<String>` — every requested URI, in order (assert on these to verify API calls)
- **Fixtures** (`provider/src/icu/test_support.rs`):
  - `icu_user(tbid, icuathlete, enabled)` — an `IcuUser` row for the store
  - `user(id)` — a domain `User` with the icu provider
  - `icu_id(n)` — an `IcuId` from a string
  - `part(id, owner, what, source)` — a domain `Part`
  - `activity_json(id, type, gear)` — canned icu activity JSON (fixed timestamp `2026-01-02T10:00:00Z`, distance `25000.0`, moving time `3300`, elevation `300.0`, joules `4000.0`)
  - `stub_activity_json(id, type)` — a stub activity (both `distance` and `moving_time` null)
  - `gear_json(id, type)` — canned icu gear JSON

## Writing new tests

```rust
use crate::test_support::{setup, queue, activity_json, icu_user};

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let (mut store, mut facade) = setup();
    store.insert_user(icu_user(UserId::from(1), "i55751783", true));
    queue("/activity/abc123", &activity_json("abc123", "Ride", Some("g1")));
    // ...
    Ok(())
}
```

- `#[tokio::test]`, return `TbResult<()>`.
- Assert on the canned `mock!` store outcomes for store state; on `requests()` for API calls.
- `setup()` calls `begin_test()` internally, so the HTTP seam is ready for `queue()` calls immediately after.

## Test files

| File | Covers |
|------|--------|
| `activity/activity_tests.rs` | Activity import: JSON → domain `Activity` mapping, type mapping, stub detection, gear reference resolution |
| `gear/gear_tests.rs` | Gear import: JSON → domain `Part` mapping, type mapping, `into_partid` upsert logic |
| `event/event_tests.rs` | Event queue: `get_event` (probe), `process` (work txn), `insert_sync`, `accept` (webhook) |
| `user/user_tests.rs` | User ops: `IcuUser::upsert`, `get_all_stats`, token management |
| `backfill/backfill_tests.rs` | Onboarding backfill: the date-window walk that queues past activities |
| `http/http_tests.rs` | HTTP seam: `request_json` against the thread-local canned responses |

## Gotchas

- `setup()` returns `(MockIcuDataStore, MockDomainFacade)` — both are owned, not references. Use `mut` bindings for store methods.
- The HTTP seam is thread-local: each `#[tokio::test]` runs on its own thread, so `begin_test()` (called by `setup()`) gives per-test isolation automatically.
- `icu_user(tbid, icuathlete, enabled)` sets `access_token` to `Some("test-token")` when `enabled` is true, `None` otherwise — the token is read by `http::request_json` to authenticate API calls.
