# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Commands (run from project root)

- `cargo build` - Build all crates (requires `SQLX_OFFLINE=true` for sqlx precompiled queries)
- `cargo run` - Run server (default: `127.0.0.1:8000`, static files from `../../frontend/dist`)
- `cargo check` - Type checking across workspace
- `SQLX_OFFLINE=true cargo build` - Required for compilation (queries cached in `.sqlx/`)
- `cargo test -p tb_domain` - Run domain unit tests (in-memory `MemStore` via `test_support`)
- `cargo run -p tb_domain --bin build-snapshot --features test-support` - Regenerate `test_support/prepopulated_data.rs` from the deterministic `snapshot()`

## Architecture

- **Workspace**: Root [`Cargo.toml`](../Cargo.toml:1) defines 5 crates: `app` (binary), `axum` (web), `domain` (business logic), `sqlx` (PostgreSQL), `strava` (API client)
- **Layering**: Clean architecture - domain traits in [`domain/src/traits/`](src/domain/src/traits/) → sqlx impls in [`sqlx/src/store/`](src/sqlx/src/store/) → axum handlers in [`axum/src/domain/`](src/axum/src/domain/)
- **Entry point**: [`app/src/main.rs`](src/app/src/main.rs:11) calls `tb_axum::start()` which sets up router, sessions, and DB pool
- **Routes**: `/api/{resource}` (user, types, shop, part, service, plan, activ) + `/strava/*` (OAuth, webhook) + root fallback to static files

## Code Conventions

- **Module layout**: Never use `mod.rs` files. Use flat file modules — declare `mod foo;` and create `foo.rs` directly, NOT `foo/mod.rs`. For example, a test support module should be `test_support.rs` not `test_support/mod.rs`. See [`lib.rs`](src/domain/src/lib.rs:25) for pattern.
- **ID types**: Newtype wrappers via `derive_more` - [`UserId`](src/domain/src/entities/user.rs:42), [`PartId`](src/domain/src/entities/part.rs:79), [`ActivityId`](src/domain/src/entities/activity.rs:43), etc.
- **Error handling**: `TbResult<T>` = `Result<T, Error>` (domain errors in [`domain/src/error.rs`](src/domain/src/error.rs:29)); `ApiResult<T>` = `Result<Json<T>, AppError>` (HTTP mapping in [`axum/src/error.rs`](src/axum/src/error.rs:24))
- **Transactions**: `let mut store = pool.begin().await?; ... store.commit().await?` - **commit is REQUIRED** after every begin()
- **CRUD pattern**: `EntityId::new(id).read(&session, &mut store).await?` or `entity.update(&session, &mut store).await?`
- **Session**: [`RequestSession`](src/axum/src/strava/session.rs:20) implements `FromRequestParts` + domain `Session` trait; carries user_id, strava_id, access_token, shop context
- **AppState**: Holds only `DbPool` - other deps extracted via `axum_macros::FromRef` derive
- **DateTime**: Serialized via `time::serde::rfc3339`; uses `OffsetDateTime` throughout
- **Clippy**: `#![warn(clippy::all)]` enforced in [`main.rs`](src/app/src/main.rs:1)

## Critical Gotchas

- **Migrations**: Auto-run via `sqlx::migrate!("./migrations")` - path relative to `sqlx/` crate dir, not project root
- **Session expiry**: 10 days inactivity (`tower_sessions::Expiry::OnInactivity(time::Duration::days(10))`)
- **tower-sessions**: Pinned to `0.14` - version `0.15` fails on deletion-task (see [`axum/Cargo.toml`](src/axum/Cargo.toml:30))
- **SQLX_OFFLINE**: Must be `true` when building - queries are pre-compiled in `.sqlx/` directory
- **DB schema**: Uses `serial` for PKs, `uuid` for usages; migrations in [`sqlx/migrations/`](src/sqlx/migrations/)
- **OnboardingStatus**: `repr(i32)` with magic values: 0=Pending, 2=Postponed, 99=Completed
- **Global allocator**: `mimalloc` with `secure` feature for performance
- **Environment vars**: `DB_URL` (default `postgres://localhost/tendabike`), `BIND_ADDR` (default `127.0.0.1:8000`), `STATIC_WWW`

## DB-to-Domain Mapping

- SQLx queries return `Db*` structs (e.g., [`DbActivity`](src/sqlx/src/store/activity.rs:8)) with DB-native types (`i32`, `i64`)
- Conversion via `impl From<DomainType> for DbType` and `impl TryFrom<DbType> for DomainType` in `sqlx/src/store/*.rs`
- Helper functions in [`sqlx/src/lib.rs`](src/sqlx/src/lib.rs:14): `vec_into()`, `option_into()`, `into_domain()`

## Domain Tests

### Running Tests

```bash
SQLX_OFFLINE=true cargo test -p tb_domain
```

All 266 tests run in-memory (no database needed). Tests are organized by entity under `domain/src/entities/*/tests/`.

### Test Tiers

| Tier | Scope | Store | Example |
|------|-------|-------|---------|
| A | Pure logic (types, enums, ID wrappers) | None | `parttypeid_is_main_bike` |
| B | Single-entity CRUD + validation | `MemStore::new()` | `user_create_and_read` |
| C | Cross-entity integration (shop, subscriptions, registration) | `MemStore::prepopulated()` | `register_part_ok` |

### Key Infrastructure

- **`MemStore`** (`test_support.rs`) — in-memory store implementing all 8 subtraits + `Store`. Use `MemStore::new()` for isolated tests or `MemStore::prepopulated()` for realistic data.
- **`TestSession`** (`test_support.rs`) — implements the `Session` trait.
  - `TestSession::new(user_id)` — customer session
  - `TestSession::with_shop(user_id, shop_id)` — shop-owner session
  - `TestSession::with_admin(user_id, true)` — admin session
- **`part_type_ids`** (`test_support.rs`) — constants: `BIKE=1`, `FRONT_WHEEL=2`, `TIRE=3`, `CHAIN=4`, `REAR_WHEEL=5`, etc.
- **`fixtures`** (`test_support/fixtures.rs`) — helpers: `fixture_basic_part()`, `fixture_attached_part()`, `fixture_assembly()`, `fixture_bike()`, `sample_purchase_date()`

### Prepopulated Snapshot

`MemStore::prepopulated()` loads a fixed JSON snapshot containing:

| Entity | Count | Details |
|--------|-------|---------|
| Users | 1 | User 1 ("Tenda"/"Bike") |
| Parts | 17 | 2 bikes + subparts (wheels, tires, chains) + 5 spares |
| Attachments | 11 | Assembly hierarchy: bike→wheel/tire/chain |
| Usages | 7 | Accumulated usage aggregates, linked from parts and attachments (and services) |
| Activities | 3 | On Bike A, for usage calculation tests |

Part ID layout: Bike A=1, Front Wheel A=2, Rear Wheel A=3, Chain A=4, Tire Front A=5, Tire Rear A=6, Bike B=7, Front Wheel B=8, Rear Wheel B=9, Chain B=10, Tire Front B=11, Tire Rear B=12, Spares=13–17 (chain 1, chain 2, tire, wheel, wheel tire).

**Important**: Part 1 (Bike A) registration cascades to parts [1, 2, 3, 4] (bike + direct subparts).

### Writing New Tests

```rust
use crate::test_support::{MemStore, TestSession, part_type_ids::*};

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let mut store = MemStore::prepopulated(); // or MemStore::new()
    let session = TestSession::new(UserId::from(1));
    // ...
    Ok(())
}
```

- Use `#[tokio::test]` for async tests
- Return `TbResult<()>` — `?` propagates domain errors with context
- For isolated tests: `MemStore::new()` + create entities explicitly
- For integration tests: `MemStore::prepopulated()` + reference existing IDs

### Snapshot Regeneration

```bash
SQLX_OFFLINE=true cargo run -p tb_domain --bin build-snapshot --features test-support
```

- Rebuilds `test_support/prepopulated_data.rs` from `build_workshop_store()` in `mem_store.rs`
- Deterministic: all collections sorted by ID; **exception**: usage UUIDs use `Uuid::now()` (v7) and differ between runs
- Do NOT edit the generated file by hand

### Test Data Rule: Do Not Modify Prepopulated Data Without Asking

- **Never modify `domain/src/test_support/prepopulated_data.rs` or its generated JSON snapshot without explicit user approval.**
- The snapshot (`SNAPSHOT_JSON`) contains a fixed set of parts, attachments, usages, and activities. Tests must adapt to this existing data — reuse available part IDs, owners, and types rather than creating new entries in the snapshot.
- When a test needs isolation (e.g., creating parts without affecting other tests), create them explicitly in the test and/or use user IDs that do not overlap with prepopulated owners (e.g., `UserId::from(98)` has no parts; `UserId::from(99)` is unused).
- The only acceptable reason to modify the snapshot is when there is an actual inconsistency between prepopulated_data.rs and the code that loads/generates it.
- After approval, rebuild it with `cargo run -p tb_domain --bin build-snapshot --features test-support` (deterministic: all collections sorted by ID) — do not edit the generated file by hand.

## Strava Tests

### Running Tests

```bash
SQLX_OFFLINE=true cargo test -p tb_strava
```

All 57 tests run in-memory (no database, no network). Tests are colocated in each module's `#[cfg(test)] mod tests`: `activity.rs`, `event.rs`, `gear.rs`, `user.rs`.

### Key Infrastructure

- **`TestStravaStore`** (`test_support.rs`) — implements `StravaStore`. Domain data delegates to the domain `MemStore` (accessible as `store.mem`, use qualified calls like `ActivityStore::get_all(&mut store.mem, &user)`); `strava_users: HashMap<UserId, StravaUser>` and `events: Vec<Event>` are plain in-memory collections with auto-assigned event IDs. `insert_user()`, `event_count()` helpers.
- **`TestStravaSession`** (`test_support.rs`) — implements `StravaSession`:
  - `queue(uri, json)` — script a JSON response body for an **exact** request URI (FIFO per URI)
  - `queue_error(uri, error)` — script an error for an exact URI; unscripted URIs yield `Error::BadRequest`
  - `requests: Vec<String>` — every requested URI, in order (assert on these to verify API calls)
  - `deauthorizes: Vec<StravaId>` — records `deauthorize()` calls
- **JSON helpers**: `activity_json(id, type, gear)` (fixed timestamp `2026-01-02T10:00:00Z` = `1767348000`, distance `25000.0`, elevation `300.0`), `gear_json(id, frame_type)` (`None` = shoes), `strava_user(tbid, stravaid, enabled)`.

### Writing New Tests

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

- Use `#[tokio::test]`, return `TbResult<()>`
- Assert on `store.events` / `store.event_count()` for webhook event state; on `session.requests` for Strava API calls

### Gotchas

- `request_json(...).await.context("...")?` wraps errors via anyhow → the propagated variant is `Error::AnyFailure`, not the original (e.g. `BadRequest`)
- `PartId.0` is private — compare via `i32::from(part_id)`
- `User` fields are `name` (lastname) and `firstname`; `MemStore::create(firstname, lastname, ...)` maps the `lastname` arg → `user.name`
- `refresh_token().map(|t| t.secret())` borrows a local — use `.map(|t| t.secret().to_string())` and compare against `Some("...".to_string())`
- Test fn names must not shadow the module functions under test (e.g. don't name a test `user_deauthorize` or `get_all_stats`) — rename to describe the behavior instead
- `42.into().method(...)` is ambiguous (i32 converts into many types) — use `StravaId::from(42)` for method calls
- `strava_event_get_next_for_user` also matches events with `owner_id == StravaId::default()` (global stop events), mirroring the SQL query
- The LSP/rust-analyzer may report stale errors in this crate (e.g. E0034 "multiple applicable items" for qualified trait paths, missing `Clone` on `ObjectType`) — trust `cargo test` / `cargo clippy` output over the LSP

## Axum Tests

### Running Tests

```bash
SQLX_OFFLINE=true cargo test -p tb_axum
```

All 40 tests run in-memory (no database; the one DB-boundary test attempts a refused TCP connect). Unit tests are colocated in each module's `#[cfg(test)] mod tests` (`error.rs`, `strava/oauth.rs`, `strava/webhook.rs`, `strava/session.rs`); router tests live in `lib.rs` `mod tests` and exercise the real router built by the private `routes()` helper.

### Coverage

- **Error mapping** (`error.rs`): every `Error` variant → expected HTTP status + body text through `AppError::into_response()`
- **OAuth helpers** (`strava/oauth.rs`): HMAC-SHA1 known-answer vector for `hmac_signature()`, `gentoken`/`getpath` roundtrip, corrupted-signature and malformed-state rejection
- **Webhook validation** (`strava/webhook.rs`): `Hub::validate()` accepts a valid subscribe, rejects wrong token / unknown mode
- **Session expiry** (`strava/session.rs`): `is_expired()` for past / future / `None` expiry
- **Router** (`lib.rs`): public endpoints (`/api/types/*`, `/strava/callback`, `/strava/login`, `/strava/token`, `/strava/logout`), 401 on unauthenticated `/api/*`, 404 (hidden) on admin-only routes for non-admins, 500 when an authenticated request reaches the DB layer, 405 wrong method, 404 unknown path

### Key Infrastructure

- **`test_support`** (`axum/src/test_support.rs`):
  - `test_app(session_store)` — full `Router` from `routes()` + `SessionManagerLayer` (mirrors `start()` minus Postgres); DB pool is a lazy `DbPool::lazy("postgres://127.0.0.1:1")` that never connects (port 1 is never listening, errors fail fast)
  - `run(app, method, uri, cookie)` — `tower::ServiceExt::oneshot` roundtrip; returns `(StatusCode, HeaderMap, Vec<u8>)`
  - `user_cookie(store)` / `admin_cookie(store)` — create a session in the store via `RequestSession::new_dummy(is_admin)`, return the `id` cookie as a header (cookie value is the session `Id`'s base64url encoding)
- **`RequestSession::new_dummy(is_admin)`** — `#[cfg(test)]` constructor in `strava/session.rs` (fields are private; always construct sessions this way in tests)
- Tests force the `CLIENT_ID` / `CLIENT_SECRET` env vars via a `Once` (edition 2024: `std::env::set_var` is `unsafe`), overriding any values from the environment so the redirect-URL assertions are deterministic; `CSRF_SECRET` is left unset (the `CSRF_KEY` falls back to a per-process random key, which is fine for token roundtrips)

### Gotchas

- Route paths have **no trailing slash**: `nest("/user") + route("/")` registers `/api/user`, not `/api/user/` (a request with the trailing slash 404s). Match the frontend, which calls `/api/user`, `/api/service`, etc.
- Auth-gated and admin-gated routes are asserted by status alone (401 `Please login` / 404) — anything past the extractors hits the memory pool and yields 500, so don't assert deeper behavior without a real database
- `DbPool::lazy()` lives in the `tb_sqlx` crate but is only exercised by tests; `start()` still uses `DbPool::new()` (connect + migrate)
- Do NOT add `pub` to `routes()` or the test helpers — the plan (and CI) relies on them staying crate-private

## SQLX Tests

### Running Tests

```bash
SQLX_OFFLINE=true cargo test -p tb_sqlx
```

All 37 tests are pure unit tests (no database, no network). Tests are colocated in each module's `#[cfg(test)] mod tests`: `lib.rs`, `tb_sqlx.rs`, `stravastore.rs`, and `store/{user,part,activity,attachment,service,serviceplan,shop,usage}.rs`.

### Coverage

- **Helpers** (`lib.rs`): `into_domain` maps `RowNotFound` → `Error::NotFound` and other sqlx errors → `Error::DatabaseFailure`; `vec_into` / `option_into` element mapping
- **Pool** (`tb_sqlx.rs`): `DbPool::lazy` panics on an invalid URL and builds a lazy pool with a 1-second acquire timeout
- **Entity mapping** (`store/*.rs`, `stravastore.rs`): domain → `Db*` field mapping (i32 IDs, UUIDs, JSON `updates`), `Db*` → domain roundtrips via `From` / `TryFrom`, and `#[should_panic]` tests documenting `unwrap()` behavior on unknown onboarding status, unknown object/aspect type, and invalid webhook `updates` JSON

### Gotchas

- All domain entities have **pub fields** — tests construct them with struct literals; no extra dev-deps are needed
- `DbPool::lazy` (sqlx `connect_lazy`) requires a Tokio context — that one test is `#[tokio::test]` (tokio is already a dev-dependency)
- `oauth2::RefreshToken` does not implement `PartialEq` — assert on `secret()` / `is_none()` instead
- The workspace `uuid` dependency has no `v4` feature — tests use a fixed `Uuid::from_str(...)` value
- Activity `utc_offset` is rounded with `((offset + 900) / 1800) * 1800` — truncation toward zero, so negative offsets shift toward zero; `UtcOffset` only accepts ±86400 s, so only offsets that round *past* that range fail
- Tests document current behavior as-is (including the `unwrap()` panics on unknown enum strings) — fixing the underlying behavior is a follow-up
