# ADR-0015: The facade module is the sole importer of the callee

## Status

Accepted. A workspace-wide access invariant: a crate reaches a callee crate only through a single facade module per (caller, callee) edge, and that module is the sole importer of the callee in the caller crate. Sibling to ADR-0012 (the executor's write path), ADR-0013 (concepts follow the dependency graph), and ADR-0014 (the provider service). It makes ADR-0013's layering invariant an *access* seam, not only a *type* seam.

## Context

ADR-0013 settled the *layering* invariant: a crate may only use concepts from crates it depends on, and the dependency graph is the complete statement of which concepts may flow where. It is enforced at the *concept* level — a leak in a string literal, a method name, or a doc comment is hidden coupling.

But the layering invariant says nothing about *access through a seam*. The facade traits exist (`ServiceApi`, `ProviderService`, `ProviderLane`, `DomainFacade`, `EntityFacade`) — they are the *type* seams a calling crate is supposed to drive. Yet a calling crate can also import concrete types, consts, and free functions directly from the called crate, bypassing the facade. The facade is a *type* seam, not an *access* seam: it constrains which *types* flow, not *which module* may name the callee at all.

The verified leaks in the workspace:

- `tb_exec` → `tb_provider`: uses `ProviderLane` *and* `use tb_provider::strava::{StravaDataStore, event::sync_users}`, `use tb_provider::Provider`.
- `tb_axum` → `tb_exec`: uses `ServiceApi` *and* `use tb_exec::{ExecutorStatus, HEARTBEAT_INTERVAL}`.
- `tb_sqlx` → `tb_provider`: `use tb_provider::strava::{StravaId, StravaUser, event::Event}`, `use tb_provider::icu::{IcuEvent, IcuId, IcuUser}`.

Each is the same hidden coupling ADR-0013 removes, one edge down: the caller names the callee's concrete surface directly, not only the seam. The dependency graph says *may*; it does not say *must go through the seam*.

## Decision

**A crate reaches a callee only through a single facade module per (caller, callee) edge.** That module is the **sole importer** of the callee in the caller crate: every other module in the caller imports from the facade module, never from the callee directly. The sole-importer invariant is machine-checked.

The facade module for an edge exports: the trait; the real calling struct and the mock struct (the mock is always named `Mock<Traitname>`); and re-exports of the domain types the trait's signatures need.

**Two categories of facades.**

*Caller-side* — the trait, the real impl, and the mock all live in the *caller* crate; the facade module holds all three. These stay caller-side: the domain remains the edgeless leaf (ADR-0013), so a caller-shaped seam does not move into the callee.

- `DomainFacade` / `DomainFacadeImpl` / `MockDomainFacade` — in `tb_exec` (caller of `tb_domain`).
- `EntityFacade` / `EntityFacadeImpl` / `MockEntityFacade` — in `tb_provider` (caller of `tb_domain`).

*Callee-side* — the trait and its impl live in the *callee* crate; the caller consumes the trait. The impl stays in the callee; the caller's facade module re-exports the trait + the mock + the domain types. The mock is generated at the trait's definition site (mockall), so for callee-side edges it stays in the callee and is re-exported through the caller's facade module as `Mock<Traitname>`.

- `ServiceApi`, `ProviderService` (+ `Registry`) — in `tb_exec` (callee for `tb_axum`).
- `ProviderLane` (+ the unified lane) — in `tb_provider` (callee for `tb_exec`).

**The sole-importer invariant and its check.** A new script, [`.githooks/check-facade-access.py`](../../.githooks/check-facade-access.py), wired into `pre-commit`, enforces the **derived sole-importer rule**: for each (caller crate `A`, callee crate `B`) pair, at most one non-exempt module in `A` may import from `B`. The facade module is simply whichever module imports from `B`; the rule needs no (A, B, module) list. The rule is universal — test modules included: the facade re-exports the test-only domain types under the same cfg gate as the test code that names them (the `test_support` modules' `cfg(any(test, feature = "test-support"))` gate, the in-crate `*_tests` modules' `cfg(test)` gate), so test code funnels through the facade too and no test exemption is needed. The check covers **service-facade edges + plain type imports**; the store-impl edges — `tb_sqlx` *implementing* the `Store` / `StravaDataStore` / `IcuDataStore` traits — run the other direction and stay under [`.githooks/check-store-access.py`](../../.githooks/check-store-access.py) (ADR-0012).

**Migration policy: reference + grandfather.** This effort conforms **both** caller-side edges as the reference implementation — their facade modules become the sole importers of `tb_domain` in their crates:

- `exec→domain`: the other `tb_exec` modules funnel through `exec::facade` (this ADR's reference) — test modules included.
- `provider→domain`: the other `tb_provider` modules funnel through `provider::facade` — test modules included.

The remaining existing crossings (the callee-side edges and the plain type imports in the other crates) are **grandfathered** via the exemption list and migrate incrementally. New crossings must conform.

## Rejected alternatives

- **Type-seam only (the status quo).** The facade trait constrains the types that flow, not the module that names the callee; a caller can import concrete types, consts, and free functions directly. Rejected: it is the same hidden coupling ADR-0013 removes, at the import level — the dependency graph says *may*, not *must go through the seam*.
- **A (caller, callee, module) allow-list.** Name the facade module for every edge in a table the check reads. Rejected: the derived rule (at most one importer) needs no table — the facade module is *whichever* module imports, so a new edge conforms by construction and the list never drifts.
- **Move the caller-side facades into the callee.** Put `DomainFacade` / `EntityFacade` in `tb_domain` so the callee owns every seam. Rejected: it makes the edgeless leaf know its callers (ADR-0013); a caller-shaped seam — the six ops an executor needs, the entity ops a provider needs — is the caller's concept and stays with the caller.
- **One facade module per crate (not per edge).** A single `facade` module that imports every callee. Rejected: it conflates the edges a crate owns (each callee is a distinct seam with its own trait and mock) and makes the sole-importer rule uncheckable per edge.

## Consequences

- `tb_exec` reaches `tb_domain` only through `exec::facade`, which re-exports the domain types the crate needs (the test-only types under the test code's own cfg gate); the other modules (`api`, `executor`, `message`, `providerservice`, `registry`, `registry/core`, `txn`, and the test modules) import those types from `crate::facade`, never from `tb_domain`. `DomainFacade` / `DomainFacadeImpl` / `MockDomainFacade` stay in `exec::facade` (caller-side; unchanged in shape).
- The sole-importer invariant is machine-checked by `.githooks/check-facade-access.py` (the check script lands with the `provider→domain` reference, after both reference edges are conformed), so a second `tb_exec` module that names `tb_domain` directly fails the pre-commit hook — a test module as much as a production one.
- The caller-side facades stay caller-side; the domain stays the edgeless leaf (ADR-0013). The callee-side facades (`ServiceApi`, `ProviderService`, `ProviderLane`) stay in the callee; the caller's facade module re-exports the trait + the mock + the domain types.
- The grandfathered crossings (the callee-side edges, the `tb_axum`→`tb_exec` plain imports, the `tb_sqlx`→`tb_provider` type imports) are covered by the exemption list and migrate incrementally; new crossings must conform.
