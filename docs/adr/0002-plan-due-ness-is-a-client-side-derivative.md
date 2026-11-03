# ADR-0002: Plan due-ness is a client-side derivative

## Status

Accepted. Supersedes [ADR-0001](archive/0001-plan-status-rule-placement.md) (withdrawn).

## Context

TendaBike tracks bike maintenance: parts carry service plans with thresholds (km, rides, days, …), and the UI shows how close each part is to being due. An earlier design moved the whole due-ness rule — remaining-threshold math, severity bands, the specific-beats-generic resolution ladder, covered-part exclusion, next-due — into the backend domain, exposing it through a module that returned a stored verdict in the `Summary` via a time-parameterized endpoint.

That placement inverts the documented domain-flow pattern. The domain computes entity state; the client combines entity state to compute *derivatives* (`docs/agents/domain-flow.md`). Due-ness is a derivative: it depends on the clock (days since the last service) as well as on accumulated usage, so it changes with the passage of time, not only on mutation. The side that owns the clock — the client — must compute it.

## Decision

The due-ness rule is a **client-side derivative** in `frontend/src/lib/serviceplan.ts`, kept behind a narrow, reactivity-preserving interface. The public doors' signatures carry only entity values and time (plus a caller-selected `plans: ServicePlan[]` where a walk is the subject); the rule reads the module's state objects inside the door functions, and the dependencies register at the enclosing reactive call site (a `$derived`, or the component's render). The rule internals — the resolution ladder, the due/alert/next-due folds, the covered-part exclusion, the gear-part resolution — are private, exposed through a small set of free-function doors plus the `ServicePlan#services` instance method. The doors return the unsorted walk; sorting is the caller's job with the exported, module-owned comparator `planCmp` (type → hook → part → id).

The backend keeps **CRUD for `Service` and `ServicePlan` plus the `unlink_plan` unlink** (`Service::unlink_plan`, called from `ServicePlan::delete` — a specific plan unlinks to the part's owner, a generic plan to the plan's `uid`) — deleting a plan removes its id from the owner's services — and nothing more: no due-ness module, no `plan_status` field on the `Summary`, no time-parameterized endpoint.

## Rejected alternatives

- **Backend rule placement.** A server-stored verdict in `Summary.plan_status` behind a time-parameterized endpoint. Rejected because a server-stored verdict is a client derivative with a clock in it: it goes stale between the poll that produced it and the render, and the client already holds the merged state and owns the clock.
- **Migrating the entity maps to reactive `$state` read by plain rule functions** (Svelte 5's "shared state" way). Initially rejected because the five entity collections are asynchronous data streams (hydration, drain and write merges) — the case the Svelte 5 store guidance points at — and the rewrite had no compensating interface gain. **That rejection no longer stands:** a later wide store-to-state migration converted the collections to `$state` objects (see [state-object.md](../agents/state-object.md)), and the rule's doors now read those state objects in their function bodies. The ADR's own decision — due-ness as a client-side derivative — is unaffected and stays accepted.

## Consequences

- `frontend/src/lib/serviceplan.ts` is the home of the rule; its public surface is the doors, the exported `planCmp`, the `Due` value object, `localizeLimitKey`, the `ServicePlan`/`Limits` shapes, `limit_keys`, the `plans` state object, and CRUD.
- Derivatives re-evaluate on state writes, not on clock passage; a midnight band flip waits for the next data change or re-hydration. Accepted and recorded.
