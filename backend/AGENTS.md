# AGENTS.md

## Commands (run from project root)

- `cargo sqlx prepare --workspace` - Regenerate the offline query metadata in `.sqlx/`

## Architecture

- **Crates**: `app` (binary), `axum` (web layer), `domain` (business logic), `sqlx` (PostgreSQL), `strava` (API client); workspace root at [`Cargo.toml`](../Cargo.toml).
- **Layering**: domain traits in [`domain/src/traits/`](domain/src/traits/) → sqlx impls in [`sqlx/src/store/`](sqlx/src/store/) → axum handlers in [`axum/src/domain/`](axum/src/domain/).

## Code Conventions

- **Module layout**: flat file modules — declare `mod foo;` with a `foo.rs` beside it; no `mod.rs`.
- **ID types**: newtype wrappers via `derive_more` (`UserId`, `PartId`, …); the inner field is private — compare/convert with `i32::from(id)`.
- **Error handling**: `TbResult<T>` = `Result<T, Error>` (domain, [`domain/src/error.rs`](domain/src/error.rs)); `ApiResult<T>` = `Result<Json<T>, AppError>` (HTTP mapping, [`axum/src/error.rs`](axum/src/error.rs)).
- **Transactions**: `let mut store = pool.begin().await?; … store.commit().await?` — commit is required after every `begin()`.

## Critical Gotchas

- **cargo sqlx**: run every `cargo sqlx` command from the project root with `--workspace` — inside a crate directory it only targets that crate and leaves the other crates' offline queries stale.
- **`tb_strava` LSP**: rust-analyzer may report stale errors in this crate (e.g. E0034 on qualified trait paths) — trust `cargo test` / `cargo clippy` output over the LSP.

## DB-to-Domain Mapping

- Helper functions in [`sqlx/src/lib.rs`](sqlx/src/lib.rs): `vec_into()`, `option_into()`, `into_domain()`.

## Tests

All suites are in-memory (no database, no network): `SQLX_OFFLINE=true cargo test -p tb_<crate>`.
Before writing or changing tests for a crate, read its guide:

- `tb_domain` → [`docs/tests/domain.md`](docs/tests/domain.md) — `MemStore`, fixtures, the prepopulated snapshot and its data rules
- `tb_strava` → [`docs/tests/strava.md`](docs/tests/strava.md) — `TestStravaStore` / `TestStravaSession`, JSON helpers
- `tb_axum` → [`docs/tests/axum.md`](docs/tests/axum.md) — `test_app`, router tests, coverage map
- `tb_sqlx` → [`docs/tests/sqlx.md`](docs/tests/sqlx.md) — entity-mapping roundtrips
