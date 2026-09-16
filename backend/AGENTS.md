# AGENTS.md

## Commands (run from project root)

- `SQLX_OFFLINE=true cargo build` - Build the workspace; `SQLX_OFFLINE=true` is required for every cargo build (queries cached in the root `.sqlx/`)
- `cargo run` - Run the server (default `127.0.0.1:8000`; serves the API and static files from `frontend/dist`)
- `cargo sqlx prepare --workspace` - Regenerate the offline query metadata in `.sqlx/`

## Architecture

- **Crates**: `app` (binary), `axum` (web layer), `domain` (business logic), `sqlx` (PostgreSQL), `strava` (API client); workspace root at [`Cargo.toml`](../Cargo.toml).
- **Layering**: domain traits in [`domain/src/traits/`](domain/src/traits/) → sqlx impls in [`sqlx/src/store/`](sqlx/src/store/) → axum handlers in [`axum/src/domain/`](axum/src/domain/).
- **Entry point**: [`app/src/main.rs`](app/src/main.rs) calls `tb_axum::start()` (router, sessions, DB pool).

## Code Conventions

- **Module layout**: flat file modules — declare `mod foo;` with a `foo.rs` beside it; no `mod.rs`.
- **ID types**: newtype wrappers via `derive_more` (`UserId`, `PartId`, …); the inner field is private — compare/convert with `i32::from(id)`.
- **Error handling**: `TbResult<T>` = `Result<T, Error>` (domain, [`domain/src/error.rs`](domain/src/error.rs)); `ApiResult<T>` = `Result<Json<T>, AppError>` (HTTP mapping, [`axum/src/error.rs`](axum/src/error.rs)).
- **Transactions**: `let mut store = pool.begin().await?; … store.commit().await?` — commit is required after every `begin()`.
- **CRUD**: `EntityId::new(id).read(&session, &mut store).await?` / `entity.update(&session, &mut store).await?`.
- **Session**: `RequestSession` ([`axum/src/strava/session.rs`](axum/src/strava/session.rs)) implements `FromRequestParts` + the domain `Session` trait; carries user_id, strava_id, access_token, shop context.
- **AppState**: holds only `DbPool`; other deps are extracted via `axum_macros::FromRef`.
- **DateTime**: `OffsetDateTime` throughout; serialized via `time::serde::rfc3339`.

## Critical Gotchas

- **Migrations**: auto-run at startup via `sqlx::migrate!("./migrations")` — the path is relative to the `sqlx/` crate dir, not the project root.
- **tower-sessions**: pinned to `0.14`; version `0.15` fails on the deletion task.
- **cargo sqlx**: run every `cargo sqlx` command from the project root with `--workspace` — inside a crate directory it only targets that crate and leaves the other crates' offline queries stale.
- **OnboardingStatus**: `repr(i32)` with magic values: 0=Pending, 2=Postponed, 99=Completed.
- **Route paths have no trailing slash**: `nest("/user") + route("/")` registers `/api/user`, not `/api/user/` — a trailing-slash request 404s; the frontend calls `/api/user`.
- **Keep `routes()` and the axum test helpers crate-private** — the router tests live in-crate.
- **`User` fields**: `name` is the lastname, `firstname` the firstname.
- **`request_json`** propagates errors as `Error::AnyFailure`, losing the original variant (e.g. `BadRequest`).
- **Webhook events**: `strava_event_get_next_for_user` also matches events with `owner_id == StravaId::default()` (global stop events), mirroring the SQL query.
- **uuid**: the workspace dependency has no `v4` feature.
- **Activity `utc_offset`**: rounded with `((offset + 900) / 1800) * 1800` — truncation toward zero; `UtcOffset` only accepts ±86400 s, so only offsets that round *past* that range fail.
- **`tb_strava` LSP**: rust-analyzer may report stale errors in this crate (e.g. E0034 on qualified trait paths) — trust `cargo test` / `cargo clippy` output over the LSP.
- **Env vars**: `DB_URL` (default `postgres://localhost/tendabike`), `BIND_ADDR` (`127.0.0.1:8000`), `STATIC_WWW` (default `../../frontend/dist`).

## DB-to-Domain Mapping

- sqlx queries return `Db*` structs with DB-native types (`i32`, `i64`).
- Conversion via `impl From<DomainType> for DbType` and `impl TryFrom<DbType> for DomainType` in `sqlx/src/store/*.rs`.
- Helper functions in [`sqlx/src/lib.rs`](sqlx/src/lib.rs): `vec_into()`, `option_into()`, `into_domain()`.

## Tests

All suites are in-memory (no database, no network): `SQLX_OFFLINE=true cargo test -p tb_<crate>`.
Before writing or changing tests for a crate, read its guide:

- `tb_domain` → [`docs/tests/domain.md`](docs/tests/domain.md) — `MemStore`, fixtures, the prepopulated snapshot and its data rules
- `tb_strava` → [`docs/tests/strava.md`](docs/tests/strava.md) — `TestStravaStore` / `TestStravaSession`, JSON helpers
- `tb_axum` → [`docs/tests/axum.md`](docs/tests/axum.md) — `test_app`, router tests, coverage map
- `tb_sqlx` → [`docs/tests/sqlx.md`](docs/tests/sqlx.md) — entity-mapping roundtrips
