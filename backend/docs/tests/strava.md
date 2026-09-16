# tb_strava tests

`SQLX_OFFLINE=true cargo test -p tb_strava`. All tests run in-memory (no database, no network); colocated in each module's `#[cfg(test)] mod tests`: `activity.rs`, `event.rs`, `gear.rs`, `user.rs`.

## Key infrastructure

- **`TestStravaStore`** (`strava/src/test_support.rs`) — implements `StravaStore`. Domain data delegates to the domain `MemStore` (accessible as `store.mem`; use qualified calls like `ActivityStore::get_all(&mut store.mem, &user)`). `strava_users: HashMap<UserId, StravaUser>` and `events: Vec<Event>` are plain in-memory collections with auto-assigned event IDs; `insert_user()`, `event_count()` helpers.
- **`TestStravaSession`** (`strava/src/test_support.rs`) — implements `StravaSession`:
  - `queue(uri, json)` — script a JSON response body for an **exact** request URI (FIFO per URI)
  - `queue_error(uri, error)` — script an error for an exact URI; unscripted URIs yield `Error::BadRequest`
  - `requests: Vec<String>` — every requested URI, in order (assert on these to verify API calls)
  - `deauthorizes: Vec<StravaId>` — records `deauthorize()` calls
- **JSON helpers**: `activity_json(id, type, gear)` (fixed timestamp `2026-01-02T10:00:00Z` = `1767348000`, distance `25000.0`, elevation `300.0`), `gear_json(id, frame_type)` (`None` = shoes), `strava_user(tbid, stravaid, enabled)`.

## Writing new tests

```rust
use crate::test_support::{TestStravaSession, TestStravaStore};

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let mut store = TestStravaStore::new();
    store.insert_user(strava_user(UserId::from(1), 42, true));
    let mut session = TestStravaSession::new(UserId::from(1), 42.into());
    session.queue("/gear/b1", &gear_json("b1", Some(0)));
    // ...
    Ok(())
}
```

- `#[tokio::test]`, return `TbResult<()>`.
- Assert on `store.events` / `store.event_count()` for webhook event state; on `session.requests` for Strava API calls.

## Gotchas

- `refresh_token().map(|t| t.secret())` borrows a local — use `.map(|t| t.secret().to_string())` and compare against `Some("...".to_string())`.
- Test fn names must not shadow the module functions under test (e.g. don't name a test `user_deauthorize` or `get_all_stats`) — name the test after the behavior instead.
- `42.into().method(...)` is ambiguous (i32 converts into many types) — use `StravaId::from(42)` for method calls.
