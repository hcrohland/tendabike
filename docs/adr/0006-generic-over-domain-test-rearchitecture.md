# ADR-0006: Test infrastructure generic over the domain

## Status

Under review (2026-10-17). The implementation (`feature/generic-domain`,
spec #525) is stopped while the spec is scrutinized; the decision is not
final. Verdict map: [#534](https://github.com/hcrohland/tendabike/issues/534).

## Context

The in-memory test store is duplicated across the backend crates. `MemStore` (`tb_domain::test_support`) is the canonical full in-memory `Store` (~78 methods over nine sub-traits). But the strava and axum suites each carry their own copy of that surface: `TestStravaStore` (`tb_strava::test_support`) and `LiveConn`/`FakeConn` (`tb_axum::test_support`) re-implement the same ~78 domain methods — by delegation to a wrapped `MemStore` or by `unimplemented!()` — just to reach the 15 `StravaStore` methods they actually exercise. That is ~995 lines of near-identical boilerplate across the strava and axum crates.

The obvious fix — share one store type across the crates — requires a separate crate that depends on the crates under test. That makes the unit-test binary link each lib twice, so the traits exist in two copies and the shared store's `impl StravaStore` (compiled against one copy) fails the `StravaStore` bound on the other (E0277). The workaround is to move the suites to `tests/` (integration tests link the rlib once), which forces the items the tests touch to be `pub`. The prior plan (spec #510, PR #519) took that route and was abandoned for widening 44 production items to `pub`; it was never merged.

On `main` the stores are already in-crate and the tests are in-src `#[cfg(test)]`, so the two-instance problem does not arise. The adopted approach removes the duplication a different way: by making `tb_strava`/`tb_exec` generic over a `Domain` trait, each consumer's test double only implements the minimal surface it drives (≈17 methods) against a **minimal test domain**, instead of carrying the full store. Concretely that deduplicates the in-memory test stores; more generally it turns every consumer's test double into a minimal stand-in for the domain, not a copy of it.

## Decision

**Make `tb_strava` and `tb_exec` generic over the domain.** A new `Domain` trait in `tb_domain` (next to `Store`) names the *entity-operation* surface strava/exec drive — associated types (`Error`, `Summary`, `Session`, `Store`, `ApiWrite`, `WriteOutcome`, the entity types) plus the operations (`upsert_activity`, `read_activity`, `delete_activity`, `create_part`, `part_by_source`, `exec`) and the constructors. The concrete `tb_domain` and a **minimal test domain** (stub operations, a ~17-method store) both satisfy it. `StravaStore`/`Txn`/`TxnSource`/`run` become generic over `D: Domain`; the `StravaStore: Store`/`Txn: Store` supertraits become combined bounds (`D::Store + StravaStore<D>`), because associated types cannot be supertraits. The tests run in-crate against the minimal test domain — one crate instance, no E0277, no `pub` widening.

As part of this, strava's one direct domain-store call (`store.partid_get_by_source`, `gear.rs:53,64`) is wrapped in a domain operation `part_by_source`, so a consumer never reaches into a foreign `Store` (reinforcing the #510 boundary rule).

The full executable spec is issue #525.

## Rejected alternatives

- **An external test-store crate (spec #510 / PR #519, abandoned).** The two-crate-instance E0277 is inherent to a double living in a separate crate that depends on the crates under test; it forces the `tests/` suites and the 44-item `pub` widening. Never merged.
- **Starting from `feature/teststore`.** Would first require undoing the crate, the `tests/` relocation, and the 44-item widening, then doing the same additive work — strictly more churn for no benefit. We start from `main`.
- **Two traits (strava surface + executor surface).** The two surfaces *share* the associated types (`Error`, `Summary`, `Session`, `Store`, the entity types), so independent traits don't compose (duplication + a "same type" constraint, or an arbitrary hierarchy). The executor also irreducibly consumes both halves (`run` processes both API writes and Strava events), so a split would not shrink the largest test domain.
- **A seam crate for `Domain`.** The trait has no implementation of its own; a crate containing only it is a shallow module. It lives in `tb_domain`, next to `Store` (which already sets the "domain crate defines the abstraction; adapters live in the consumer crates" pattern).
- **Trait objects (`dyn Domain`) / GATs.** `dyn Domain` requires fixing every associated type on the object and switches to dynamic dispatch, losing the concrete types strava/exec construct — the point of the genericity. GATs aren't needed (no associated type depends on a lifetime/type parameter).

## Consequences

- One new `pub` item: the `Domain` trait in `tb_domain` (it must be `pub` to appear in strava/exec's public bounds). All 44 previously widened items return to `pub(crate)`; the production API is not widened.
- The strava/axum in-memory test stores shrink from ~93 to ~17 methods (15 `StravaStore` + 2 no-op `Txn`); the ~78 delegating `Store` sub-trait methods disappear because the stub domain ops never touch the store.
- Every `tb_strava`/`tb_exec` signature that names a domain type gains a `D: Domain` parameter and `D::` prefixes; the `run`/`queue_probe`/`run_write`/`run_event` bounds gain `D::Store`. Mechanical and reviewable.
- The six #510 carry-forward decisions (no transaction buffer, the `Arc<Mutex<…>>` lock, the panic arm, the retired `Fault`/`fail_next`, the boundary rule) are preserved, adapted to the in-crate + generic structure.
- `tb_sqlx` (which depends on the concrete `tb_domain`) and its scratch seeder are unaffected.
