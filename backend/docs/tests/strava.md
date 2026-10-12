# tb_strava tests

`SQLX_OFFLINE=true cargo test -p tb_strava`. All tests run in-memory (no database, no network); one integration test per module under `strava/tests/` (`activity_tests.rs`, `event_tests.rs`, `gear_tests.rs`, `user_tests.rs`). They are integration tests (not in-src `#[cfg(test)]` modules) because the store double, `tb_teststore::TestStore`, depends on `tb_strava` — only an integration test links the lib rlib once, so `TestStore`'s `StravaStore` impl is the same trait instance the code under test is bound to (spec #510, amendment).

## Key infrastructure

- **`TestStore`** (`tb_teststore`, dev-dependency) — the unified in-memory store (spec #510): one shared state, the nine `Store` sub-traits and `StravaStore` implemented directly, no transaction buffer (every write lands immediately; `commit`/`rollback` are no-ops). Seeding: `TestStore::new()` (empty) or `TestStore::prepopulated()` (the workshop snapshot); Strava user rows are seeded with `store.insert_user(strava_user(...))`.
- **Assertions** — domain data goes through the `Store` traits directly on the store (e.g. `store.get_all(&user)`, `store.partid_get_part(pid)` — import the sub-trait you call, e.g. `tb_domain::ActivityStore`); the Strava half through `state()` (e.g. `store.state().strava.events`, `store.state().strava.users`), `event_count()`, and the `StravaStore` methods.
- **`TestStravaSession`** (`strava/src/test_support.rs`, a public always-compiled module) — implements `StravaSession`:
  - `queue(uri, json)` — script a JSON response body for an **exact** request URI (FIFO per URI)
  - `queue_error(uri, error)` — script an error for an exact URI; unscripted URIs yield `Error::BadRequest`
  - `requests: Vec<String>` — every requested URI, in order (assert on these to verify API calls)
  - `deauthorizes: Vec<StravaId>` — records `deauthorize()` calls
- **JSON helpers**: `activity_json(id, type, gear)` (fixed timestamp `2026-01-02T10:00:00Z` = `1767348000`, distance `25000.0`, elevation `300.0`), `gear_json(id, frame_type)` (`None` = shoes), `strava_user(tbid, stravaid, enabled)`.

## Writing new tests

```rust
use tb_strava::test_support::{TestStravaSession, gear_json, strava_user};
use tb_teststore::TestStore;

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let mut store = TestStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let mut session = TestStravaSession::new(UserId::from(1), 42.into());
    session.queue("/gear/b1", &gear_json("b1", Some(0)));
    // ...
    Ok(())
}
```

- Tests live in `strava/tests/`; there is no `use super::*` — import what you need from `tb_strava` and `tb_domain`, including the `Store` sub-traits you call on the store.
- `#[tokio::test]`, return `TbResult<()>`.
- Assert on `store.state().strava.events` / `store.event_count()` for webhook event state; on `session.requests` for Strava API calls.
- When you need a reference into `state()` that outlives the statement, bind the state first: `let state = store.state();` (the clone is a temporary otherwise).

## Gotchas

- `refresh_token().map(|t| t.secret())` borrows a local — use `.map(|t| t.secret().to_string())` and compare against `Some("...".to_string())`.
- Test fn names must not shadow the module functions under test (e.g. don't name a test `user_deauthorize` or `get_all_stats`) — name the test after the behavior instead.
- `42.into().method(...)` is ambiguous (i32 converts into many types) — use `StravaId::from(42)` for method calls.
