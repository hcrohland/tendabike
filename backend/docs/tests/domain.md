# tb_domain tests

`SQLX_OFFLINE=true cargo test -p tb_domain`. All tests run in-memory (no database); organized by entity under `domain/src/entities/*/tests/`.

## Test tiers

| Tier | Scope | Store | Example |
|------|-------|-------|---------|
| A | Pure logic (types, enums, ID wrappers) | None | `parttypeid_is_main_bike` |
| B | Single-entity CRUD + validation | `MemStore::new()` | `user_create_and_read` |
| C | Cross-entity integration (shop, subscriptions, registration) | `MemStore::prepopulated()` | `register_part_ok` |

## Key infrastructure

- **`MemStore`** (`domain/src/test_support.rs`) — in-memory store implementing all 8 subtraits + `Store`. Use `MemStore::new()` for isolated tests, `MemStore::prepopulated()` for realistic data.
- **`TestSession`** (`domain/src/test_support.rs`) — implements the `Session` trait: `new(user_id)` (customer), `with_shop(user_id, shop_id)` (shop owner), `with_admin(user_id, true)` (admin).
- **`part_type_ids`** (`domain/src/test_support.rs`) — constants: `BIKE=1`, `FRONT_WHEEL=2`, `TIRE=3`, `CHAIN=4`, `REAR_WHEEL=5`, etc.
- **`fixtures`** (`domain/src/test_support/fixtures.rs`) — helpers: `fixture_basic_part()`, `fixture_attached_part()`, `fixture_assembly()`, `fixture_bike()`, `sample_purchase_date()`.

## The prepopulated snapshot

`MemStore::prepopulated()` loads a fixed JSON snapshot:

| Entity | Count | Details |
|--------|-------|---------|
| Users | 1 | User 1 ("Tenda"/"Bike") |
| Parts | 17 | 2 bikes + subparts (wheels, tires, chains) + 5 spares |
| Attachments | 11 | Assembly hierarchy: bike→wheel/tire/chain |
| Usages | 7 | Accumulated usage aggregates, linked from parts and attachments (and services) |
| Activities | 3 | On Bike A, for usage calculation tests |

Part ID layout: Bike A=1, Front Wheel A=2, Rear Wheel A=3, Chain A=4, Tire Front A=5, Tire Rear A=6, Bike B=7, Front Wheel B=8, Rear Wheel B=9, Chain B=10, Tire Front B=11, Tire Rear B=12, Spares=13–17 (chain 1, chain 2, tire, wheel, wheel tire).

Part 1 (Bike A) registration cascades to parts [1, 2, 3, 4] (bike + direct subparts).

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

## Snapshot regeneration

```bash
SQLX_OFFLINE=true cargo run -p tb_domain --bin build-snapshot --features test-support
```

Rebuilds `test_support/prepopulated_data.rs` from `build_workshop_store()` in `mem_store.rs`. Deterministic: all collections sorted by ID; **exception**: usage UUIDs use `Uuid::now()` (v7) and differ between runs. Rebuild rather than editing the generated file by hand.
