# AGENTS.md

## Commands

- `cargo sqlx prepare` (run from `backend/sqlx/`) - Regenerate the crate-local offline query metadata in `backend/sqlx/.sqlx/`

## Architecture

- **Crates**: `app` (binary), `axum` (web layer), `domain` (business logic), `exec` (per-user executor — the `Txn`/`TxnSource` transaction seam), `sqlx` (PostgreSQL), `strava` (API client); workspace root at [`Cargo.toml`](../Cargo.toml).
- **Layering**: domain traits in [`domain/src/traits/`](domain/src/traits/) → sqlx impls in [`sqlx/src/store/`](sqlx/src/store/) → axum handlers in [`axum/src/domain/`](axum/src/domain/); spec #446 §1 edges: `exec` → `domain` + `strava`, `sqlx` → `exec`, `axum` → `exec` (never `tb_sqlx` — it takes the `PgPool` for its `PostgresStore` session store). **The entity owns its store (ADR-0010)**: a store method (`<entity>_<op>`) is called only from its owning entity's code; a cross-entity need goes through the callee's entity method, never its store. Enforced by `.githooks/check-store-access.py`.

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

The DB-free suites (domain, strava, axum) run against `mockall` `mock!` doubles; the `#[ignore]`d seam suites in `tb_sqlx` run against a real Postgres and are the behavioral bar — they need `--include-ignored` and `SCRATCH_DATABASE_URL`.

```bash
SQLX_OFFLINE=true cargo test -q -- --include-ignored   # complete suite (seam included)
SQLX_OFFLINE=true cargo test -q -p tb_<crate>          # one crate's DB-free suites
```

Before writing, changing, or exploring tests for a crate, read its guide:

- `tb_domain` → [`docs/tests/domain.md`](docs/tests/domain.md) — `MockStore` (the `mock!` store double), fixtures, the prepopulated snapshot and its data rules
- `tb_strava` → [`docs/tests/strava.md`](docs/tests/strava.md) — `MockStravaConn` (the `mock!` store double), `strava_session`/`strava_admin_session`, JSON helpers
- `tb_axum` → [`docs/tests/axum.md`](docs/tests/axum.md) — `test_app`, router tests, coverage map
- `tb_exec` → [`docs/tests/exec.md`](docs/tests/exec.md) — `MockConn` (the `mock!` store/`Txn` witness), the loop's control-flow units, the frame-push/orchestration ownership boundary
- `tb_sqlx` → [`docs/tests/sqlx.md`](docs/tests/sqlx.md) — entity-mapping roundtrips, the seam integration suites
