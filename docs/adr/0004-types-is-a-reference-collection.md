# Types are a reference collection

## Status

Accepted. Records a wont-do from the 2026-09-29 architecture review, which had proposed unifying the `types` collection into `mapableState` ("make types a first-class collection").

## Context

`types` (`frontend/src/lib/types.ts`) is a plain module-level `Map<Type>`: `getTypes()` fetches `GET /api/types/part` and `GET /api/types/activity` once at startup (App awaits it before mounting the router), merges the activity types onto the part types, and never writes the map again. The `/types` routes are read-only — no endpoint creates, updates, or deletes a type. The five other entity collections are `mapableState` records because they are asynchronous data streams: hydration replaces them, the drain and every write merge into them mid-session. `types` is none of those things. It is reference data, loaded once.

## Decision

`types` stays a plain map, outside `mapableState`. The unification is a wont-do, and the plain reference map is the sanctioned fourth shape, documented in `docs/agents/state-object.md`.

`mapableState`'s interface — the `setMap`/`updateMap`/`deleteItem` op keys, `stateValues` filtering, the prep/del hooks — exists to do mid-session merge and delete work. For a collection that changes exactly once per session, that interface is surface with no behaviour behind it. The costs of the plain map are real and accepted: tests seed it through the `getTypes()` fetch stub instead of one `setMap` call, `mapable.svelte.ts` keeps its plain-Map overloads (`stateValues`/`filterValues`), and `Part/ChangePart.svelte` carries a load-timing comment.

## Consequences

- The map is not reactive. That is harmless while no endpoint can change types; if a types-mutating endpoint is ever added, this ADR is superseded first — at that point the collection must track.
- Architecture reviews and agent work should not re-propose unifying `types` into `mapableState` while this ADR stands.
