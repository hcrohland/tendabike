# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Commands (run from project root)

- `cargo build` - Build all crates (requires `SQLX_OFFLINE=true` for sqlx precompiled queries)
- `cargo run` - Run server (default: `127.0.0.1:8000`, static files from `../../frontend/dist`)
- `cargo check` - Type checking across workspace
- `SQLX_OFFLINE=true cargo build` - Required for compilation (queries cached in `.sqlx/`)

**No test framework is configured** - `doctest = false` and `test = false` on domain/sqlx/strava crates.

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
