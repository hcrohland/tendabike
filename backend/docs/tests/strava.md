# tb_strava tests

All tests run in-memory (no database, no network); one `<name>_tests.rs` module per module under `strava/src/<name>/`, declared `#[cfg(test)]` in `activity.rs`, `event.rs`, `gear.rs`, `user.rs`.

## Behavior ownership

`tb_strava` owns the **Strava → domain mapping**: translating Strava JSON into domain entities (gear JSON → `Part`, activity JSON → `Activity`, user JSON → `StravaUser`) and the Strava-logic free functions over it (activity sync, gear hydration, event processing, deauthorization).

- **What it mocks** — the **domain** (the strava → domain seam): `MockStravaConn` accumulates the `Store` seam + the `StravaStore` surface, and the free functions take a bare `&mut impl StravaStore`, so the domain calls are canned. The **Strava HTTP API** is stubbed behind the thread-local canned-response map (`queue` / `queue_error` / `requests`).
- **What it delegates to the seam** — the **store I/O** (real Postgres) lives in the sqlx seam suites (see [`sqlx.md`](sqlx.md)); the **frame-push / orchestration** (deciding what to push after hydration) lives in `tb_exec` (see [`exec.md`](exec.md)); the **domain-level results** live in the `tb_domain` suite (see [`domain.md`](domain.md)).

## Key infrastructure

- **`MockStravaConn`** (`strava/src/test_support.rs`) — a `mockall` `mock!` witness (`#[cfg(test)]`) accumulating the full `Store` seam (nine sub-traits + the method-free `Store` marker) plus the `StravaStore` surface; the Strava-logic free functions take a bare `&mut impl StravaStore`, so no `Txn`. The Strava-logic tests (into_partid, activity sync, gear hydration, event processing, deauthorize) run the logic for real and assert on canned `mock!` outcomes.
- **`strava_session` / `strava_admin_session`** (`strava/src/test_support.rs`) — build a domain `MockSession` that seeds the `TokenCache` with a warm token (no refresh), so the thin free functions' refresh-token persistence is a no-op for it. The canned responses live in a **thread-local** map the free functions consult (each test runs on its own thread → per-test isolation); `strava_session` begins that scope. The canned-response surface is free functions (not methods on the session):
  - `queue(uri, json)` — script a JSON response body for an **exact** request URI (FIFO per URI)
  - `queue_error(uri, error)` — script an error for an exact URI; unscripted URIs yield `Error::BadRequest`
  - `requests() -> Vec<String>` — every requested URI, in order (assert on these to verify API calls)
  - `deauthorizes() -> Vec<StravaId>` — records `deauthorize()` calls
- **JSON helpers**: `activity_json(id, type, gear)` (fixed timestamp `2026-01-02T10:00:00Z` = `1767348000`, distance `25000.0`, elevation `300.0`), `gear_json(id, frame_type)` (`None` = shoes), `strava_user(tbid, stravaid, enabled)`.

## Writing new tests

```rust
use crate::test_support::{TestStravaStore, queue, strava_session};

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let mut session = strava_session(UserId::from(1), 42.into());
    queue("/gear/b1", &gear_json("b1", Some(0)));
    // ...
    Ok(())
}
```

- `#[tokio::test]`, return `TbResult<()>`.
- Assert on the canned `mock!` store outcomes for store state; on `requests()` for Strava API calls.
- The `CANNED_STRAVA` global (`test_support::canned_strava(uri, json)`, feature `test-support`) is a *separate*, process-wide offline seam for the `tb_sqlx` executor-seam suite, whose session runs in a spawned task on a thread the thread-local map never reaches. The thin free functions consult the thread-local map first, then `CANNED_STRAVA`.

## Gotchas

- `refresh_token().map(|t| t.secret())` borrows a local — use `.map(|t| t.secret().to_string())` and compare against `Some("...".to_string())`.
- Test fn names must not shadow the module functions under test (e.g. don't name a test `user_deauthorize` or `get_all_stats`) — name the test after the behavior instead.
- `42.into().method(...)` is ambiguous (i32 converts into many types) — use `StravaId::from(42)` for method calls.
