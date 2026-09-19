# ADR-0002: Plan due-ness is a client-side derivative

## Status

Accepted. Supersedes ADR-0001 (*Plan Status Rule Placement and Two-Door Interface*), which lived on the unmerged `misc` branch and is withdrawn with this ADR.

## Context

TendaBike tracks bike maintenance: parts carry service plans with thresholds (km, rides, days, …), and the UI shows how close each part is to being due. ADR-0001 moved the whole due-ness rule — remaining-threshold math, severity bands, the specific-beats-generic resolution ladder, covered-part exclusion, next-due — into the backend domain (`backend/domain/src/planstatus.rs`), exposing it through a two-door interface with verdicts riding in the `Summary` and a time-parameterized endpoint.

That placement inverts the documented domain-flow pattern. The domain computes entity state; the client combines entity state to compute *derivatives* (`docs/agents/domain-flow.md`). Due-ness is a derivative: it depends on the clock (days since the last service) as well as on accumulated usage, so it changes with the passage of time, not only on mutation. The side that owns the clock — the client — must compute it.

## Decision

The due-ness rule stays a **client-side derivative** in `frontend/src/lib/serviceplan.ts`, deepened in place behind a narrow, reactivity-preserving interface. Each call site reads exactly the stores its door reads, inside its own derived; the rule internals (the ladder, the comparator with the hook-branch fix, the due/alert/next-due folds, the covered-part exclusion) are private, exposed through eight free-function doors. The interface is locked in map ticket [#315](https://github.com/hcrohland/tendabike/issues/315); the executable spec [#323](https://github.com/hcrohland/tendabike/issues/323) is the record of that lock.

The backend keeps **CRUD for `Service` and `ServicePlan` plus the `reset_plan` unlink** — deleting a plan removes its id from the owner's services — and nothing more: no due-ness module, no `plan_status` field on the `Summary`, no time-parameterized endpoint. The unlink is the self-contained salvage from the `misc` work (map ticket [#314](https://github.com/hcrohland/tendabike/issues/314)).

## Rejected alternatives

- **Backend rule placement (ADR-0001).** The two-door `planstatus` module, verdicts in `Summary.plan_status`, and the `GET /api/plan/{part_id}/at` endpoint. Rejected because a server-stored verdict is a client derivative with a clock in it: it goes stale between the poll that produced it and the render, and the client already holds the merged state and owns the clock.
- **The S door — lib-layer runes-state migration.** Migrate the five entity maps (`plans`, `parts`, `services`, `usages`, `attachments`) to `.svelte.ts` `$state` read by plain rule functions — Svelte 5's "shared state" way. Rejected on the async-data-streams ground: these collections are complex asynchronous data streams (hydration fetch, 60s drain merges, write merges through `mapable`), which is exactly the case the Svelte 5 stores guidance points at, and the migration is a 29-reader/4-writer rewrite with no compensating interface gain. The reactivity objection that originally motivated the question does not hold — reading a `.svelte.ts` `$state` value inside a plain function *does* register a dependency at a `$derived` call site, verified cross-module and cross-component in [#322](https://github.com/hcrohland/tendabike/issues/322) — so the rejection rests on the async-data-streams ground alone.

## Consequences

- `frontend/src/lib/serviceplan.ts` remains the home of the rule; its public surface narrows to the eight doors plus the entity/`Limits` shapes and CRUD (spec #323 is the execution record; the `reset_plan` unlink lands separately).
- Derivatives re-evaluate on store writes, not on clock passage; a midnight band flip waits for the next data change or re-hydration. Accepted and recorded.
- ADR-0001 (on `misc`) is withdrawn with this ADR; its salvageable piece is the `reset_plan` unlink.
