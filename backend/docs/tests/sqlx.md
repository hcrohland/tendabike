# tb_sqlx tests

`SQLX_OFFLINE=true cargo test -p tb_sqlx`. The unit tests are pure (no database, no network), colocated in each module's `#[cfg(test)] mod tests`: `lib.rs`, `tb_sqlx.rs`, `stravastore.rs`, and `store/{user,part,activity,attachment,service,serviceplan,shop,usage}.rs`. The store-seam integration suite (`tests/store_seam.rs`) is the one exception: it runs against a real Postgres (below).

## Coverage

- **Helpers** (`lib.rs`): `into_domain` maps `RowNotFound` → `Error::NotFound` and other sqlx errors → `Error::DatabaseFailure`; `vec_into` / `option_into` element mapping
- **Pool** (`tb_sqlx.rs`): `DbPool::lazy` panics on an invalid URL and builds a lazy pool with a 1-second acquire timeout
- **Entity mapping** (`store/*.rs`, `stravastore.rs`): domain → `Db*` field mapping (i32 IDs, UUIDs, JSON `updates`), `Db*` → domain roundtrips via `From` / `TryFrom`, and `#[should_panic]` tests documenting `unwrap()` behavior on unknown onboarding status, unknown object/aspect type, and invalid webhook `updates` JSON

## The store-seam integration suite (`tests/store_seam.rs`)

The one non-in-memory suite (issue #406). It loads the standard prepopulated fixture — the same snapshot the `tb_domain` in-memory suite runs against — into a real Postgres scratch database it manages itself, drives a representative set of domain operations through `SqlxConn` (attachment attach, merge, and delete; activity upsert, update, delete, and lookup; the user summary read), and asserts the same domain-level results the in-memory suite asserts. The contract — one rule per operation, verified on both store adapters, with the Postgres behavior as the source of truth — is recorded in [`docs/agents/domain-flow.md`](../../../docs/agents/domain-flow.md) ("The store seam").

- **Ignored by default, run explicitly.** Every test carries `#[ignore = "requires SCRATCH_DATABASE_URL (a scratch Postgres database)"]`, so a plain `SQLX_OFFLINE=true cargo test --workspace` run reports the whole suite ignored with that reason, executes none of it, and stays green without a database. The suite runs with `cargo test -- --include-ignored` (the `--` matters: `--include-ignored` is a libtest flag, not a cargo flag).
- **Fail loud, never skip.** Whenever the suite cannot run it fails instead of skipping: with no `SCRATCH_DATABASE_URL` (after the `.env` fallback) every test fails with the no-URL message, and with the URL set but the scratch database un-preparable (setup, pool, or transaction) every test fails with the one root-cause message naming the step. A test that verified nothing never passes. `database_is_reachable` is the named canary: it additionally bounds the one-time setup with a 10s guard, so a blackholed endpoint fails in seconds.
- **Scratch lifecycle.** On first use the suite force-drops any database a previous run left behind, creates a fresh one, runs the migrations through the same `DbPool::new` the app uses, and seeds the fixture; it drops nothing at the end of a run, so every run — including a rerun after a failure — starts from the same clean slate. The URL names the scratch database, and its user needs createdb rights on that server.
- **CI.** The required `rust` job in `.github/workflows/test.yml` (issue #411) carries a `postgres:18` service, sets `SCRATCH_DATABASE_URL` to a scratch database on it, pins `SQLX_OFFLINE=true` (the suite creates the scratch database at runtime and it gets its schema from the pool's migrations, so compile-time query checks must use the committed `.sqlx` offline cache), and runs `cargo test --release -- --include-ignored`, so the seam suite runs in the one required gate alongside the in-memory suites. A red seam blocks the PR.
- **Determinism.** All tests run serialized against one freshly created scratch database: a one-time seed loads the fixture into it (committed once; the database is empty by construction, so no truncate is needed), then each test opens its own transaction with the sequences reset just past the fixture ids and rolls back, so every test starts from exactly the prepopulated state.

The suite never reads `DATABASE_URL` — that variable points at the developer's working database, and the old `DATABASE_URL`-based design seeded it in place, destroying real data on a local run. Run it locally from the repo root with a disposable database and the explicit flag (`backend/sqlx/.env` may supply `SCRATCH_DATABASE_URL` too; without it the tests fail with the no-URL message):

```bash
SCRATCH_DATABASE_URL=postgres://user@localhost/tendabike_test cargo test -p tb_sqlx --test store_seam -- --include-ignored
```

## Gotchas

- All domain entities have **pub fields** — tests construct them with struct literals; no extra dev-deps are needed.
- `DbPool::lazy` (sqlx `connect_lazy`) requires a Tokio context — that one test is `#[tokio::test]` (tokio is already a dev-dependency).
- `oauth2::RefreshToken` does not implement `PartialEq` — assert on `secret()` / `is_none()` instead.
- UUIDs in tests are fixed `Uuid::from_str(...)` values.
- Tests document current behavior as-is (including the `unwrap()` panics on unknown enum strings) — fixing the underlying behavior is a follow-up.
