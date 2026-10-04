# tb_domain tests

`SQLX_OFFLINE=true cargo test -p tb_domain`. All tests run in-memory (no database); organized by entity under `domain/src/entities/*/tests/`.

## Test tiers

| Tier | Scope | Store | Example |
|------|-------|-------|---------|
| A | Pure logic (types, enums, ID wrappers) | None | `parttypeid_is_main_bike` |
| B | Single-entity CRUD + validation | `MemStore::new()` | `user_create_and_read` |
| C | Cross-entity integration (shop, subscriptions, registration) | `MemStore::prepopulated()` | `register_part_ok` |

## Key infrastructure

- **`MemStore`** (`domain/src/test_support.rs`) — in-memory store implementing all nine sub-traits + `Store`. Use `MemStore::new()` for isolated tests, `MemStore::prepopulated()` for realistic data.

### Transactional semantics (issue #409)

A `MemStore` is a **transaction** on an in-memory database, mirroring the production `SqlxConn` (an open Postgres transaction):

- Every write lands in this transaction's working copy; reads see the working copy when it exists, otherwise the committed state. Single-transaction tests behave exactly like the old eager store.
- `store.commit()` (an inherent method on `MemStore`) merges the working copy into the database; the store is consumed, like `SqlxConn::commit`.
- Dropping the store without committing, or `store.rollback().await?`, discards all uncommitted writes (the store stays usable, back at the committed state). `SqlxConn::rollback` consumes the store instead — a deliberate ergonomic difference.
- `store.begin()` opens a **sibling transaction** on the same database. Siblings see only committed state: uncommitted writes of the other transaction are invisible until it commits, and vanish when it is dropped or rolled back. This is the only way to observe commit/abort.
- Siblings with their own pending writes read their own snapshot (repeatable-read style); they do not see the other transaction's commits until they roll back or a fresh `begin()` is used. Good enough for tests — Postgres is READ COMMITTED.

`store.state()` / `store.state_mut()` expose the `StoreData` working copy (all tables + id counters) for assertions and the fault-injection tests; fixtures build through the store's interface instead (below). `store.snapshot()` returns the ordered `StoreSnapshot`.

**Fault injection** — `store.fail_next(Fault::X)` makes the next call of kind `X` fail with `Error::DatabaseFailure`, like a real database failure, so tests can drive a multi-write domain operation to fail mid-transaction and then `rollback()` (see `failed_attach_rolls_back_to_committed_state` in `mem_store.rs` for the pattern). Current kinds: `Fault::AttachmentCreate`, `Fault::UsageUpdate`.
- **`TestSession`** (`domain/src/test_support.rs`) — implements the `Session` trait: `new(user_id)` (customer), `with_shop(user_id, shop_id)` (shop owner), `with_admin(user_id, true)` (admin).
- **`part_type_ids`** (`domain/src/test_support.rs`) — constants: `BIKE=1`, `FRONT_WHEEL=2`, `TIRE=3`, `CHAIN=4`, `REAR_WHEEL=5`, etc.
- **`fixtures`** (`domain/src/test_support/fixtures.rs`) — helpers: `fixture_basic_part()`, `fixture_attached_part()`, `fixture_attached_part_to_gear()`, `fixture_attached_part_at()`, `fixture_assembly()`, `fixture_timeline()`, `fixture_concurrent_parts()`, `fixture_bike()`, `sample_purchase_date()`, `sample_attach_time()`.

  Built through the store's documented interface (issue #410):

  - **Construction** — parts via `Part::create`, attachments via `attach_assembly`; results are read back through the store's trait methods (`attachments_all_by_part`). Nothing in the module touches `store.state()`. The one direct-write exception: `fixture_assembly` writes two overlapping tire rows through `attachment_create` — two parts of the same type at the same hook overlap in time, a state the attach operation never produces (it replaces the hook's occupant; the state-space decision recorded on `attach_assembly`, issue #407, where the database allows the overlap).
  - **Fixed dates** — every fixture time is a fixed value; the suite has no wall clock. `sample_purchase_date()` is `1700000000` (2023-11-14), other purchases are offsets from it, and `sample_attach_time()` is 30 days after it.
  - **Determinism** — two independent builds of the same scenario produce identical data. The one exception is usage UUIDs: the domain operations mint them via `Uuid::now_v7()` on every run (the same model the snapshot documents). The determinism test `prepopulated_fixture_is_deterministic_across_builds` canonicalizes those UUIDs and asserts the rest matches byte for byte.

## The prepopulated snapshot

`MemStore::prepopulated()` loads a fixed JSON snapshot:

| Entity | Count | Details |
|--------|-------|---------|
| Users | 1 | User 1 ("Tenda"/"Bike") |
| Parts | 17 | 2 bikes + subparts (wheels, tires, chains) + 5 spares |
| Attachments | 11 | Assembly hierarchy: bike→wheel/tire/chain. Tires on mounted wheels carry the bike as gear with the wheel as hook (flat row model); the spare wheel tire carries the loose wheel |
| Usages | 11 | Accumulated usage aggregates for Bike A's set (bike + 5 attached parts, linked from parts and attachments) |
| Activities | 3 | On Bike A, for usage calculation tests |

Part ID layout: Bike A=1, Front Wheel A=2, Rear Wheel A=3, Chain A=4, Tire Front A=5, Tire Rear A=6, Bike B=7, Front Wheel B=8, Rear Wheel B=9, Chain B=10, Tire Front B=11, Tire Rear B=12, Spares=13–17 (chain 1, chain 2, tire, wheel, wheel tire).

Part 1 (Bike A) registration cascades to parts [1, 2, 3, 4, 5, 6] (bike + direct subparts, tires included).

**How the snapshot is built and loaded.** Every row in the snapshot comes from domain operations — `build_workshop_store()` in `mem_store.rs` runs `Part::create`, `attach_assembly`, the activity `upsert`s, and `Activity::rescan_all` on a fresh store, all with fixed dates; no state-space exceptions are needed (no same-type overlaps). The generated JSON is loaded through the store's admin path, the one place data reaches the store without a domain operation, mirroring a production bootstrap: `MemStore::prepopulated()` seeds the in-memory database, and the store-seam suite (`sqlx/tests/store_seam.rs`) seeds the database with SQL. The checked-in file is static, so the prepopulated data is byte-identical across runs (regenerating it mints fresh v7 usage UUIDs — see below).

### Ask before changing snapshot data

- `domain/src/test_support/prepopulated_data.rs` and its generated JSON snapshot (`SNAPSHOT_JSON`) are fixed; tests adapt to this data — reuse the available part IDs, owners, and types.
- When a test needs isolation, create entities explicitly in the test and/or use user IDs that do not overlap the prepopulated owners (`UserId::from(98)` has no parts; `UserId::from(99)` is unused).
- The only reason to change the snapshot is an actual inconsistency between `prepopulated_data.rs` and the code that loads/generates it.
- After user approval, rebuild with the snapshot regeneration below.

## Writing new tests

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

- `#[tokio::test]` for async; return `TbResult<()>` so `?` propagates domain errors with context.
- Isolated tests: `MemStore::new()` + create entities explicitly. Integration tests: `MemStore::prepopulated()` + reference existing IDs.
- `MemStore::create(firstname, lastname, ...)` maps the `lastname` arg to `user.name` (remember: `User.name` is the lastname).
- Store behavior tests (keying, normalization, commit/abort) live in the `mem_*.rs` files, not the entity test modules; domain tests stay domain-focused.
- The in-memory attachment key is the database's `(part_id, attached_time)` primary key: a duplicate insert fails with `Error::DatabaseFailure`, mirroring the production store.
- Activities come back with their offset rounded to the nearest 30 minutes (the production read rule, documented on `Activity`); assert on the instant (`unix_timestamp()`), not the offset label, unless the rounding itself is what you test.

## Snapshot regeneration

```bash
SQLX_OFFLINE=true cargo run -p tb_domain --bin build-snapshot --features test-support
```

Rebuilds `test_support/prepopulated_data.rs` from `build_workshop_store()` in `mem_store.rs`. Deterministic: all collections sorted by ID; **exception**: usage UUIDs use `Uuid::now()` (v7) and differ between runs. Rebuild rather than editing the generated file by hand.
