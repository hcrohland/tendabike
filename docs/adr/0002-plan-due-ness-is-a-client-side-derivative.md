# ADR-0002: Plan due-ness is a client-side derivative

## Status

Accepted. Supersedes ADR-0001 (*Plan Status Rule Placement and Two-Door Interface*), which lived on the unmerged `misc` branch and is withdrawn with this ADR.

## Context

TendaBike tracks bike maintenance: parts carry service plans with thresholds (km, rides, days, …), and the UI shows how close each part is to being due. ADR-0001 moved the whole due-ness rule — remaining-threshold math, severity bands, the specific-beats-generic resolution ladder, covered-part exclusion, next-due — into the backend domain (`backend/domain/src/planstatus.rs`), exposing it through a two-door interface with verdicts riding in the `Summary` and a time-parameterized endpoint.

That placement inverts the documented domain-flow pattern. The domain computes entity state; the client combines entity state to compute *derivatives* (`docs/agents/domain-flow.md`). Due-ness is a derivative: it depends on the clock (days since the last service) as well as on accumulated usage, so it changes with the passage of time, not only on mutation. The side that owns the clock — the client — must compute it.

## Decision

The due-ness rule stays a **client-side derivative** in `frontend/src/lib/serviceplan.ts`, deepened in place behind a narrow, reactivity-preserving interface. Each call site passes exactly the state objects its door reads; the rule reads them inside plain functions, and the dependencies register at the enclosing reactive call site (a `$derived`, or the component's render). The rule internals (the resolution ladder, the due/alert/next-due folds, the covered-part exclusion, the gear-part resolution) are private, exposed through eight free-function doors. The doors return the unsorted walk; sorting is the caller's job with the exported, module-owned comparator `planCmp` (type → hook → part → id, with the PlanList hook-branch bug fixed). The interface is locked in map ticket [#315](https://github.com/hcrohland/tendabike/issues/315); the executable spec [#323](https://github.com/hcrohland/tendabike/issues/323) is the record of that lock, amended with the three contract decisions confirmed in [#328](https://github.com/hcrohland/tendabike/issues/328): unsorted walks, exported `planCmp`, boolean `isTemplate`.

The backend keeps **CRUD for `Service` and `ServicePlan` plus the `unlink_plan` unlink** (`Service::unlink_plan`, called from `ServicePlan::delete` — specific plan → the part's owner, generic plan → the plan's `uid`) — deleting a plan removes its id from the owner's services — and nothing more: no due-ness module, no `plan_status` field on the `Summary`, no time-parameterized endpoint. The unlink is the self-contained salvage from the `misc` work (map ticket [#314](https://github.com/hcrohland/tendabike/issues/314)).

## Rejected alternatives

- **Backend rule placement (ADR-0001).** The two-door `planstatus` module, verdicts in `Summary.plan_status`, and the `GET /api/plan/{part_id}/at` endpoint. Rejected because a server-stored verdict is a client derivative with a clock in it: it goes stale between the poll that produced it and the render, and the client already holds the merged state and owns the clock.
- **The S door — lib-layer runes-state migration.** Migrate the five entity maps (`plans`, `parts`, `services`, `usages`, `attachments`) to `.svelte.ts` `$state` read by plain rule functions — Svelte 5's "shared state" way. Rejected on the async-data-streams ground: these collections are complex asynchronous data streams (hydration fetch, 60s drain merges, write merges through `mapable`), which is exactly the case the Svelte 5 stores guidance points at, and the migration is a 29-reader/4-writer rewrite with no compensating interface gain. The reactivity objection that originally motivated the question does not hold — reading a `.svelte.ts` `$state` value inside a plain function *does* register a dependency at a `$derived` call site, verified cross-module and cross-component in [#322](https://github.com/hcrohland/tendabike/issues/322) — so the rejection rests on the async-data-streams ground alone. (The prototype's original "missing reactivity" observation was a test-context artifact: a plain read outside a reactive scope is a snapshot, and a module-scope `$derived` cannot be exported.)

  **Superseded by the wide store-to-state migration.** The [#346](https://github.com/hcrohland/tendabike/issues/346) probe — the usages collection converted to a `mapableState` state object as a controlled experiment — re-verified the reactivity ground in the live app (tracking behaved exactly as [#322](https://github.com/hcrohland/tendabike/issues/322) had predicted) and its review approved the wider migration, [#349](https://github.com/hcrohland/tendabike/issues/349). That migration executed with a minimal reader diff and zero write-path change: the five maps are now `$state` collections produced by the `.svelte.ts` `mapableState` factory, and the plan module's doors take those state objects directly — the S door as realized. The rejection no longer stands on the grounds recorded above: the async-data-streams ground and the expected 29-reader/4-writer cost were undercut by the migration. The ADR's own decision — due-ness as a client-side derivative — is unaffected and stays accepted.

## Consequences

- `frontend/src/lib/serviceplan.ts` remains the home of the rule; its public surface narrows to the eight doors, the exported `planCmp`, and the `ServicePlan`/`Limits` shapes, `limit_keys`, the `plans` state object, and CRUD (spec #323 is the execution record; the `unlink_plan` unlink landed as its backend half).
- Derivatives re-evaluate on state writes, not on clock passage; a midnight band flip waits for the next data change or re-hydration. Accepted and recorded.
- ADR-0001 (on `misc`) is withdrawn with this ADR; its salvageable piece is the `unlink_plan` unlink.
