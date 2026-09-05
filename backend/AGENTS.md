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
