# tb_sqlx tests

The unit tests are pure (no database, no network), colocated in each module's `#[cfg(test)] mod tests`: `lib.rs`, `tb_sqlx.rs`, `stravastore.rs`, and `store/{user,part,activity,attachment,service,serviceplan,shop,usage}.rs`. Two integration suites are the exceptions: `tests/store_seam.rs` (store I/O) and `tests/executor_seam.rs` (the executor's transaction lifecycle) — both run against a real Postgres (below).

## Coverage

- **Helpers** (`lib.rs`): `into_domain` maps `RowNotFound` → `Error::NotFound` and other sqlx errors → `Error::DatabaseFailure`; `vec_into` / `option_into` element mapping
- **Pool** (`tb_sqlx.rs`): `DbPool::lazy` panics on an invalid URL and builds a lazy pool with a 1-second acquire timeout
- **Entity mapping** (`store/*.rs`, `stravastore.rs`): domain → `Db*` field mapping (i32 IDs, UUIDs, JSON `updates`), `Db*` → domain roundtrips via `From` / `TryFrom`, and `#[should_panic]` tests documenting `unwrap()` behavior on unknown onboarding status, unknown object/aspect type, and invalid webhook `updates` JSON

## The store-seam integration suite (`tests/store_seam.rs`)

`tb_sqlx`'s `store_seam.rs` owns **store I/O and nothing else** (issue #406): the SQL clauses, the adapter mappings, the entity roundtrips, and the scratch canary — the store-level facts, verified on real Postgres. It loads the standard prepopulated fixture — the same snapshot the `tb_domain` in-memory suite runs against — into a real Postgres scratch database it manages itself, and drives representative store paths through `SqlxConn` asserting the store-level outcome (the PK violation → `DatabaseFailure`, the `RowNotFound` → `NotFound`, the `ORDER BY`, the `start >=` / `< end` range boundary, the `Db*` → domain roundtrip). It does **not** re-assert the domain-level results — those are owned by the in-memory `tb_domain` suite (see [`domain.md`](domain.md)). The contract — one store-level fact per path, with the Postgres behavior as the source of truth — is recorded in [`docs/agents/domain-flow.md`](../../../docs/agents/domain-flow.md) ("The store seam").

- **What it mocks** — nothing at the store level: it runs real Postgres, and the domain logic runs for real over the live `SqlxConn` adapter (the Postgres adapter itself is the boundary under test).
- **What it delegates to the seam** — the domain-level results (the `Summary`, the `TbResult`): owned by the in-memory `tb_domain` suite (see [`domain.md`](domain.md)); this suite asserts only the store-level fact, never re-asserting a domain result.

- **Ignored by default, run explicitly.** Every test carries `#[ignore]`, so a plain workspace run reports the whole suite ignored, executes none of it, and stays green without a database. The suite runs with `--include-ignored` (the `--` matters: it's a libtest flag, not a cargo flag).
- **Fail loud, never skip.** Whenever the suite cannot run it fails instead of skipping: with no `SCRATCH_DATABASE_URL` (after the `.env` fallback) every test fails with the no-URL message, and with the URL set but the scratch database un-preparable (setup, pool, or transaction) every test fails with the one root-cause message naming the step. A test that verified nothing never passes. `database_is_reachable` is the named canary: it additionally bounds the one-time setup with a 10s guard, so a blackholed endpoint fails in seconds.
- **Scratch lifecycle.** On first use the suite force-drops any database a previous run left behind, creates a fresh one, runs the migrations through the same `DbPool::new` the app uses, and seeds the fixture; it drops nothing at the end of a run, so every run — including a rerun after a failure — starts from the same clean slate. The URL names the scratch database, and its user needs createdb rights on that server.
- **Determinism.** All tests run serialized against one freshly created scratch database: a one-time seed loads the fixture into it (committed once; the database is empty by construction, so no truncate is needed), then each test opens its own transaction with the sequences reset just past the fixture ids and rolls back, so every test starts from exactly the prepopulated state.

The suite never connects to `DATABASE_URL` or `DB_URL` — those variables point at the developer's working database, and the old `DATABASE_URL`-based design seeded it in place, destroying real data on a local run. It reads the two variables only to refuse a collision: a scratch URL that equals either fails the run instead of dropping real data. Run it locally from the repo root with the explicit flag. It reads `SCRATCH_DATABASE_URL` from the environment (set in the devcontainer, or `backend/sqlx/.env`) and fails with the no-URL message if unset:

```bash
cargo test -q -p tb_sqlx --test store_seam -- --include-ignored
```

## The executor-seam integration suite (`tests/executor_seam.rs`)

`tb_sqlx`'s `executor_seam.rs` owns the **executor's transaction lifecycle on the live pool** — the store I/O at the executor boundary that `tb_exec`'s unit tests stand in for: commit visibility, rollback, FIFO order per user, the pool/queue-probe interaction, and the reclaim-on-idle policy. It drives the real executor loop — `tb_exec::LoopContext` over the real `DbPool` (`TxnSource`/`Txn` implemented on the live `SqlxConn`) — against real Postgres, asserting the store-level fact on the live adapter (the committed row visible to a fresh connection, the rolled-back write leaving no partial row, the FIFO read seeing the earlier write, the queue drained by the probe).

- **What it mocks** — the **`Session`** is stubbed (a canned `MockSession`), and **Strava** is stubbed behind the `CANNED_STRAVA` offline seam for the gear-hydration path; the store itself is real Postgres.
- **What it delegates to the seam** — the domain-level results are owned by the in-memory `tb_domain` suite (see [`domain.md`](domain.md)), and the frame-push / orchestration logic is owned by `tb_exec` (see [`exec.md`](exec.md)); this suite asserts only the store-level outcome on the live pool, never re-asserting a domain frame/summary result.

- **Ignored by default, run explicitly.** Like the store-seam suite, every test carries `#[ignore]` and the suite runs with `--include-ignored` + a scratch Postgres URL.
- **Suffixed scratch database.** The suite works on a **suffixed** scratch database (`<scratch>_exec`), not the store-seam suite's: cargo runs test binaries in parallel, and each suite's one-time setup force-drops the database its URL names, so two suites on one URL would drop each other's database out from under the run. The lifecycle is otherwise shared through `common/scratch.rs` (the same canary, the same one-time setup, the same lock).
- **Per-test reset.** The executor loop **commits** its messages, so a committed row outlives the test; each test therefore starts from a truncated-and-re-seeded scratch database (`scratch::fresh_fixture`), the same state the unit tests see — unlike the store-seam suite, which rolls every test back.
- **Fail loud, never skip.** Without `SCRATCH_DATABASE_URL` every test fails with the no-URL message, and a scratch database that cannot be prepared fails every test with the one root-cause string.

```bash
cargo test -q -p tb_sqlx --test executor_seam -- --include-ignored
```

## Gotchas

- All domain entities have **pub fields** — tests construct them with struct literals; no extra dev-deps are needed.
- `DbPool::lazy` (sqlx `connect_lazy`) requires a Tokio context — that one test is `#[tokio::test]` (tokio is already a dev-dependency).
- `oauth2::RefreshToken` does not implement `PartialEq` — assert on `secret()` / `is_none()` instead.
- UUIDs in tests are fixed `Uuid::from_str(...)` values.
- Tests document current behavior as-is (including the `unwrap()` panics on unknown enum strings) — fixing the underlying behavior is a follow-up.
