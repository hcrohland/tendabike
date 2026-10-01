# tb_sqlx tests

`SQLX_OFFLINE=true cargo test -p tb_sqlx`. The unit tests are pure (no database, no network), colocated in each module's `#[cfg(test)] mod tests`: `lib.rs`, `tb_sqlx.rs`, `stravastore.rs`, and `store/{user,part,activity,attachment,service,serviceplan,shop,usage}.rs`. The store-seam integration suite (`tests/store_seam.rs`) is the one exception: it runs against a real Postgres (below).

## Coverage

- **Helpers** (`lib.rs`): `into_domain` maps `RowNotFound` → `Error::NotFound` and other sqlx errors → `Error::DatabaseFailure`; `vec_into` / `option_into` element mapping
- **Pool** (`tb_sqlx.rs`): `DbPool::lazy` panics on an invalid URL and builds a lazy pool with a 1-second acquire timeout
- **Entity mapping** (`store/*.rs`, `stravastore.rs`): domain → `Db*` field mapping (i32 IDs, UUIDs, JSON `updates`), `Db*` → domain roundtrips via `From` / `TryFrom`, and `#[should_panic]` tests documenting `unwrap()` behavior on unknown onboarding status, unknown object/aspect type, and invalid webhook `updates` JSON

## The store-seam integration suite (`tests/store_seam.rs`)

The one non-in-memory suite (issue #406). It loads the standard prepopulated fixture — the same snapshot the `tb_domain` in-memory suite runs against — into a real Postgres scratch database it manages itself, drives a representative set of domain operations through `SqlxConn` (attachment attach, merge, and delete; activity upsert, update, delete, and lookup; the user summary read), and asserts the same domain-level results the in-memory suite asserts. The contract — one rule per operation, verified on both store adapters, with the Postgres behavior as the source of truth — is recorded in [`docs/agents/domain-flow.md`](../../../docs/agents/domain-flow.md) ("The store seam").

- **Self-skip.** With no `SCRATCH_DATABASE_URL` configured (the plain `rust` CI job, and a local machine without a scratch database) every test skips itself, so `SQLX_OFFLINE=true cargo test --workspace` stays green without a database.
- **Fail-fast.** `database_is_reachable` does not skip when `SCRATCH_DATABASE_URL` is set: it fails loudly when the scratch database cannot be prepared within 10s — the required `postgres-seam` job sets the variable unconditionally, so it cannot pass with every test silently skipped against a dead Postgres.
- **Scratch lifecycle.** On first use the suite force-drops any database a previous run left behind, creates a fresh one, runs the migrations through the same `DbPool::new` the app uses, and seeds the fixture; it drops nothing at the end of a run, so every run — including a rerun after a failure — starts from the same clean slate. The URL names the scratch database, and its user needs createdb rights on that server.
- **CI.** The required `postgres-seam` job in `.github/workflows/test.yml` (issue #411) runs the suite against a `postgres:16` service with `SQLX_OFFLINE=true`: the suite creates the scratch database at runtime and it gets its schema from the pool's migrations, so compile-time query checks must use the committed `.sqlx` offline cache. A red seam blocks the PR.
- **Determinism.** All tests run serialized against one freshly created scratch database: a one-time seed loads the fixture into it (committed once; the database is empty by construction, so no truncate is needed), then each test opens its own transaction with the sequences reset just past the fixture ids and rolls back, so every test starts from exactly the prepopulated state.

The suite never reads `DATABASE_URL` — that variable points at the developer's working database, and the old `DATABASE_URL`-based design seeded it in place, destroying real data on a local run. Run it locally from the repo root with a disposable database (`backend/sqlx/.env` may supply `SCRATCH_DATABASE_URL` too; without it the suite skips itself):

```bash
SCRATCH_DATABASE_URL=postgres://user@localhost/tendabike_test cargo test -p tb_sqlx --test store_seam
```

## Gotchas

- All domain entities have **pub fields** — tests construct them with struct literals; no extra dev-deps are needed.
- `DbPool::lazy` (sqlx `connect_lazy`) requires a Tokio context — that one test is `#[tokio::test]` (tokio is already a dev-dependency).
- `oauth2::RefreshToken` does not implement `PartialEq` — assert on `secret()` / `is_none()` instead.
- UUIDs in tests are fixed `Uuid::from_str(...)` values.
- Tests document current behavior as-is (including the `unwrap()` panics on unknown enum strings) — fixing the underlying behavior is a follow-up.
