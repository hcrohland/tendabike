# tb_domain tests

`SQLX_OFFLINE=true cargo test -p tb_domain`. All tests run in-memory (no database); one integration test per area under `domain/tests/` (`activity_tests.rs`, `attachment_tests.rs`, `part_tests.rs`, `partnote_tests.rs`, `service_tests.rs`, `serviceplan_tests.rs`, `shop_tests.rs`, `subscription_tests.rs`, `types_tests.rs`, `usage_tests.rs`, `user_tests.rs`, `apiwrite_tests.rs`, `summary_tests.rs`).

They are integration tests (not in-src `#[cfg(test)]` modules) because the store double, `tb_teststore::TestStore`, depends on `tb_domain` — an in-src unit test would link two instances of the crate (the `--test` source build and the rlib inside `tb_teststore`), so `TestStore`'s `Store` impls would not type-check. Only an integration test links the lib rlib once (spec #510, amendment — the same treatment `tb_strava` got).

## Test tiers

| Tier | Scope | Store | Example |
|------|-------|-------|---------|
| A | Pure logic (types, enums, ID wrappers) | None | `parttypeid_is_main_bike` |
| B | Single-entity CRUD + validation | `TestStore::new()` | `user_create_and_read` |
| C | Cross-entity integration (shop, subscriptions, registration) | `TestStore::prepopulated()` | `register_part_ok` |

## Key infrastructure

- **`TestStore`** (`tb_teststore`, dev-dependency) — the unified in-memory store (spec #510): one shared state, the nine `Store` sub-traits, `StravaStore`, `TxnSource` and `Txn` implemented directly. **No transaction buffer** — every write lands immediately in the shared state, and `commit()`/`rollback()` are consuming no-ops (the old working-copy/begin/abort semantics and fault injection are retired, spec #510/#504). Use `TestStore::new()` for isolated tests, `TestStore::prepopulated()` for realistic data; `store.state()` hands back a clone of the shared state for post-write assertions.
- **`TestSession`** (`tb_domain::test_support`) — implements the `Session` trait: `new(user_id)` (customer), `with_shop(user_id, shop_id)` (shop owner), `with_admin(user_id, true)` (admin).
- **`part_type_ids`** (`tb_domain::test_support`) — constants: `BIKE=1`, `FRONT_WHEEL=2`, `TIRE=3`, `CHAIN=4`, `REAR_WHEEL=5`, etc.
- **`only_tombstone` / `sole_live`** (`tb_domain::test_support`) — `Summary` assertion helpers: `only_tombstone(id)` builds a one-entry tombstone map for whole-map assertions; `sole_live(&map)` clones out the map's single live entry.
- **`fixtures`** (`tb_teststore::fixtures`) — helpers: `fixture_basic_part()`, `fixture_attached_part()`, `fixture_attached_part_to_gear()`, `fixture_attached_part_at()`, `fixture_assembly()`, `fixture_timeline()`, `fixture_concurrent_parts()`, `fixture_bike()`, `sample_purchase_date()`, `test_session()`, `test_user()`.

  Built through the store's documented interface (issue #410):

  - **Construction** — parts via `Part::create`, attachments via `attach_assembly`; results are read back through the store's trait methods (`attachments_all_by_part`). Nothing in the module touches `store.state()`. The one direct-write exception: `fixture_assembly` writes two overlapping tire rows through `attachment_create` — two parts of the same type at the same hook overlap in time, a state the attach operation never produces (it replaces the hook's occupant; the state-space decision recorded on `attach_assembly`, issue #407, where the database allows the overlap).
  - **Fixed dates** — every fixture time is a fixed value; the suite has no wall clock. `sample_purchase_date()` is `1700000000` (2023-11-14), other purchases are offsets from it, and the sample attach time is 30 days after it.
  - **Determinism** — two independent builds of the same scenario produce identical data. The one exception is usage UUIDs: the domain operations mint them via `Uuid::now_v7()` on every run (the same model the snapshot documents). The determinism test `prepopulated_fixture_is_deterministic_across_builds` (in `tb_teststore`) canonicalizes those UUIDs and asserts the rest matches byte for byte.

  Store-behavior tests (offset normalization, `ORDER BY start`, the `(part_id, attached)` key, duplicate-id rejection) live in `tb_teststore` next to the impls (`store_activity.rs`, `store_attachment.rs`), not in the entity test modules — domain tests stay domain-focused.

## The prepopulated snapshot

`TestStore::prepopulated()` loads a fixed JSON snapshot:

| Entity | Count | Details |
|--------|-------|---------|
| Users | 1 | User 1 ("Tenda"/"Bike") |
| Parts | 17 | 2 bikes + subparts (wheels, tires, chains) + 5 spares |
| Attachments | 11 | Assembly hierarchy: bike→wheel/tire/chain. Tires on mounted wheels carry the bike as gear with the wheel as hook (flat row model); the spare wheel tire carries the loose wheel |
| Usages | 11 | Accumulated usage aggregates for Bike A's set (bike + 5 attached parts, linked from parts and attachments) |
| Activities | 3 | On Bike A, for usage calculation tests |

Part ID layout: Bike A=1, Front Wheel A=2, Rear Wheel A=3, Chain A=4, Tire Front A=5, Tire Rear A=6, Bike B=7, Front Wheel B=8, Rear Wheel B=9, Chain B=10, Tire Front B=11, Tire Rear B=12, Spares=13–17 (chain 1, chain 2, tire, wheel, wheel tire).

Part 1 (Bike A) registration cascades to parts [1, 2, 3, 4, 5, 6] (bike + direct subparts, tires included).

**How the snapshot is built and loaded.** Every row in the snapshot comes from domain operations — `build_workshop_store()` in `tb_teststore` runs `Part::create`, `attach_assembly`, the activity `upsert`s, and `Activity::rescan_all` on a fresh store, all with fixed dates; no state-space exceptions are needed (no same-type overlaps). The generated JSON is loaded through the store's admin path, the one place data reaches the store without a domain operation, mirroring a production bootstrap: `TestStore::prepopulated()` seeds the in-memory store, and the store-seam suite (`sqlx/tests/store_seam.rs`) seeds the database with SQL. The checked-in file is static, so the prepopulated data is byte-identical across runs (regenerating it mints fresh v7 usage UUIDs — see below).

### Ask before changing snapshot data

- `teststore/src/prepopulated_data.rs` and its generated JSON snapshot (`SNAPSHOT_JSON`) are fixed; tests adapt to this data — reuse the available part IDs, owners, and types.
- When a test needs isolation, create entities explicitly in the test and/or use user IDs that do not overlap the prepopulated owners (`UserId::from(98)` has no parts; `UserId::from(99)` is unused).
- The only reason to change the snapshot is an actual inconsistency between `prepopulated_data.rs` and the code that loads/generates it.
- After user approval, rebuild with the snapshot regeneration below.

## Writing new tests

```rust
use tb_domain::test_support::{TestSession, part_type_ids::*};
use tb_domain::*;
use tb_teststore::TestStore;

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let mut store = TestStore::prepopulated(); // or TestStore::new()
    let session = TestSession::new(UserId::from(1));
    // ...
    Ok(())
}
```

- Tests live in `domain/tests/`; there is no `use super::*` — import what you need from `tb_domain` (including the `Store` sub-traits you call on the store, and `test_support::{TestSession, part_type_ids, only_tombstone, sole_live}`), plus `tb_teststore::TestStore` and `tb_teststore::fixtures`.
- `#[tokio::test]` for async; return `TbResult<()>` so `?` propagates domain errors with context.
- Isolated tests: `TestStore::new()` + create entities explicitly. Integration tests: `TestStore::prepopulated()` + reference existing IDs.
- `User::create(firstname, lastname, ...)` maps the `lastname` arg to `user.name` (remember: `User.name` is the lastname).
- The in-memory attachment key is the database's `(part_id, attached_time)` primary key: a duplicate insert fails with `Error::DatabaseFailure`, mirroring the production store.
- Activities come back with their offset rounded to the nearest 30 minutes (the production read rule, documented on `Activity`); assert on the instant (`unix_timestamp()`), not the offset label, unless the rounding itself is what you test.

## Snapshot regeneration

```bash
SQLX_OFFLINE=true cargo run -p tb_teststore --bin build-snapshot
```

Rebuilds `teststore/src/prepopulated_data.rs` from `build_workshop_store()` in `teststore/src/snapshot.rs`. Deterministic: all collections sorted by ID; **exception**: usage UUIDs use `Uuid::now()` (v7) and differ between runs. Rebuild rather than editing the generated file by hand.
