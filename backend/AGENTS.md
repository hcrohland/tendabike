# AGENTS.md

## Commands

- `cargo sqlx prepare` (run from `backend/sqlx/`) - Regenerate the crate-local offline query metadata in `backend/sqlx/.sqlx/`

## Architecture

- **Crates**: `app` (binary), `axum` (web layer), `domain` (business logic), `exec` (per-user executor — the `Txn`/`TxnSource` transaction seam), `sqlx` (PostgreSQL), `strava` (API client); workspace root at [`Cargo.toml`](../Cargo.toml).
- **Layering**: domain traits in [`domain/src/traits/`](domain/src/traits/) → sqlx impls in [`sqlx/src/store/`](sqlx/src/store/) → axum handlers in [`axum/src/domain/`](axum/src/domain/); spec #446 §1 edges: `exec` → `domain` + `strava`, `sqlx` → `exec`, `axum` → `exec` (never `tb_sqlx` — it takes the `PgPool` for its `PostgresStore` session store).

## Code Conventions

- **Module layout**: flat file modules — declare `mod foo;` with a `foo.rs` beside it; no `mod.rs`.
- **ID types**: newtype wrappers via `derive_more` (`UserId`, `PartId`, …); the inner field is private — compare/convert with `i32::from(id)`.
- **Error handling**: `TbResult<T>` = `Result<T, Error>` (domain, [`domain/src/error.rs`](domain/src/error.rs)); `ApiResult<T>` = `Result<Json<T>, AppError>` (HTTP mapping, [`axum/src/error.rs`](axum/src/error.rs)).
- **Transactions**: `let mut store = pool.begin().await?; … store.commit().await?` — commit is required after every `begin()`.

## Critical Gotchas

- **cargo sqlx**: run every `cargo sqlx` command from inside `backend/sqlx/` — only `tb_sqlx` uses the query macros, and its offline metadata is crate-local (`backend/sqlx/.sqlx/`); `--workspace` would write it to the workspace root instead.
- **`tb_strava` LSP**: rust-analyzer may report stale errors in this crate (e.g. E0034 on qualified trait paths) — trust `cargo test` / `cargo clippy` output over the LSP.

## DB-to-Domain Mapping

- Helper functions in [`sqlx/src/lib.rs`](sqlx/src/lib.rs): `vec_into()`, `option_into()`, `into_domain()`.

## Tests

Most suites are in-memory; the `#[ignore]`d seam suites in `tb_sqlx` need `--include-ignored` and `SCRATCH_DATABASE_URL`.

```bash
SQLX_OFFLINE=true cargo test -q -- --include-ignored   # complete suite (seam included)
SQLX_OFFLINE=true cargo test -q -p tb_<crate>          # one crate's in-memory suites
```

Before writing or changing tests for a crate, read its guide:

- `tb_domain` → [`docs/tests/domain.md`](docs/tests/domain.md) — `MemStore`, fixtures, the prepopulated snapshot and its data rules
- `tb_strava` → [`docs/tests/strava.md`](docs/tests/strava.md) — `TestStravaStore` / `TestStravaSession`, JSON helpers
- `tb_axum` → [`docs/tests/axum.md`](docs/tests/axum.md) — `test_app`, router tests, coverage map
- `tb_sqlx` → [`docs/tests/sqlx.md`](docs/tests/sqlx.md) — entity-mapping roundtrips, the seam integration suites
