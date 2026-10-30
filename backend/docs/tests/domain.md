# tb_domain tests

All `tb_domain` tests run DB-free (no database). One `<name>_tests.rs` module per entity under `domain/src/entities/<name>/`.

## Behavior ownership

`tb_domain` owns the **domain-level results**: pure value-logic needs no store, and every store-touching test runs the real domain operation over a `mockall` `mock!` double, [`MockStore`], asserting the domain result (the `Summary`, the `TbResult`). The domain logic runs for real; only the store I/O the operation calls is scripted.

- **What it mocks** — the **store trait seam**: [`MockStore`] accumulates the full `Store` surface (the nine sub-traits + the method-free `Store` marker). The double implements the trait; it never re-implements store behavior.
- **What it delegates to the seam** — store *behavior*: keying, normalization, commit/abort, `RowNotFound` mapping, the read/write roundtrips. That is not duplicated here; it is verified once on the real Postgres path, the sqlx seam suite (`sqlx/tests/store_seam.rs`), which seeds a scratch database from the checked-in `SNAPSHOT_JSON` fixture and drives representative store paths through `SqlxConn`. It is the real-Postgres behavioral bar for database-specific behavior (PK violations, ordering, `RowNotFound` mapping). It is `#[ignore]`d by default and runs with `--include-ignored` + a scratch Postgres URL. See [`sqlx.md`](sqlx.md) for the run recipe and the seam contract (recorded in [`docs/agents/domain-flow.md`](../../../docs/agents/domain-flow.md), "The store seam").

## Test tiers

| Tier | Scope | Store | Example |
|------|-------|-------|---------|
| A | Pure logic (types, enums, ID wrappers, field projection) | None | `parttypeid_is_main_bike` |
| B | Single-entity CRUD + validation (domain logic over canned rows) | `MockStore` | `partid_is_main_for_bike` |
| C | Cross-entity integration (dispose/restore, registration, usage aggregation) | `MockStore` | `part_dispose_restores_usage` |

The `MockStore` is a canned dependency: the domain logic runs for real, only the store I/O is scripted. Tests that would exercise the *database's* behavior (the `PartId::read`/`name` lossless roundtrip, `RowNotFound` → `NotFound`, the 30-minute activity offset rounding) are not run here — they live on the sqlx seam.

## Key infrastructure

All in `domain/src/test_support.rs` (the `#[cfg(any(test, feature = "test-support"))]` module).

- **`MockStore`** (`domain/src/test_support/mock_store.rs`) — a `mockall` `mock!` witness accumulating the full `Store` seam (the nine sub-traits + the method-free `Store` marker) on one type. Build one with `MockStore::new()` and script each method the operation calls: `store.expect_partid_get_part().returning(...)`. `#[async_trait]` on each impl block is what makes the `async fn` methods desugar; the seam is collision-free (post-recipe: renamed methods, `usages_update(&[Usage])`, `stravauser_update_token` owning its token), so the flat `expect_<name>` API reaches every method.
- **`MockSession`** (`domain/src/traits.rs`) — the `Session` trait carries `#[automock]` under `test-support`, so every test session is a mockall `MockSession` with a fixed identity (`check_owner` is wired to the trait default). Build one with the `mock_session(user)` (customer), `mock_session_with_shop(user, shop)` (shop owner), or `mock_session_admin(user)` (admin) helpers in `domain/src/test_support.rs`.
- **`part_type_ids`** (`domain/src/test_support.rs`) — constants: `BIKE=1`, `FRONT_WHEEL=2`, `TIRE=3`, `CHAIN=4`, `REAR_WHEEL=5`, etc.
- **`fixtures`** (`domain/src/test_support/fixtures.rs`) — store-agnostic helpers: `test_user()` (a fixed `UserId`), `test_session()` (a `MockSession` for it), and `sample_purchase_date()` (a fixed `OffsetDateTime`). The suites have no wall clock, so every fixture time is a fixed value.
- **`only_tombstone(id)` / `sole_live(map)`** (`domain/src/test_support.rs`) — summary assertion helpers: `only_tombstone` builds an `IdKeyedMap` holding exactly the tombstone for `id` (whole-map assertions), and `sole_live` clones out the one live entry (asserting the op reported exactly one entity).
- **`StoreSnapshot`** (re-exported from `domain/src/snapshot.rs`) — the standalone serializable shape of the prepopulated fixture. It is not test-gated: the sqlx seam suites load the checked-in `SNAPSHOT_JSON` fixture by direct deserialization into it (no store round-trip).

## Writing new tests

```rust
use crate::test_support::{
    MockStore,
    fixtures::{sample_purchase_date, test_session, test_user},
    mock_session,
    part_type_ids::BIKE,
};

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let mut store = MockStore::new();
    store
        .expect_partid_get_part()
        .returning(move |_| Ok(/* a canned Part row */));
    let session = mock_session(test_user());
    // ... run the real domain operation against `store`, assert on the result
    Ok(())
}
```

- `#[tokio::test]` for async; return `TbResult<()>` so `?` propagates domain errors with context.
- Script only the store methods the operation under test calls (`MockStore` has no state — an unscripted call panics, which is a test bug, not a store bug).
- Canned rows are built as struct literals (all domain entities have pub fields) via `sample_purchase_date()` for their times.
- Pure value-logic (a field projection, a static-table lookup with no I/O) needs no `MockStore` at all.
- Where the rule is about the *database* (roundtrips, `RowNotFound`, offset rounding), do not re-implement it here — rest on the sqlx seam.

## Fixture regeneration

The checked-in fixture (`SNAPSHOT_JSON` in `domain/src/prepopulated_data.rs`) is regenerated by the `build-snapshot` bin, which runs the canonical workshop's domain-operation sequence against a scratch Postgres database, reads the rows back into a `StoreSnapshot`, and serializes them. Codegen requires a scratch Postgres database, consistent with the `#[ignore]`d seam suites:

```bash
cargo run -p tb_domain --bin build-snapshot --features test-support
```

Deterministic except for the v7 usage UUIDs, which are minted on every run. Rebuild rather than editing the generated file by hand.
