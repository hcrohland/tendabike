# ADR-0013: Concepts follow the dependency graph

## Status

Accepted. A workspace-wide layering invariant: a crate may only use concepts from crates it depends on. `tb_domain` is the extreme case — it depends on nothing, so it is ignorant of its entire environment.

## Context

The backend is a workspace of six crates with a strict dependency direction:

```
app    → axum, exec, sqlx
axum   → domain, strava, exec
exec   → domain, strava
sqlx   → domain, strava, exec
strava → domain
domain → ∅
```

`tb_domain` is the leaf: the business core with no outgoing edges. The dependency graph was always the *intended* statement of which concepts may flow where — a crate reaches another only by depending on it. But the invariant was not enforced at the *value* level: a crate could name a concept belonging to a crate it does not depend on, without the compiler complaining, because the leak lives in a string literal, a method name, or a doc comment — not a type import.

`#761` (the `ActivityId` type change) exposed this. To make `ActivityId` a provider-prefixed string it added the prefix knowledge *into the domain*: `ActivityId::strava(123)`, `ActivityId::icu("…")`, and `ActivityId::strava_id()`. The domain — the edgeless leaf — now named two specific providers and parsed one of their prefixes. The same leak sat one edge up: `tb_exec`'s `activity_url` discriminated the provider itself (`match id.strava_id() { … }`), and a persistence fixture (`SNAPSHOT_JSON` / `StoreSnapshot`) lived in `tb_domain` though only `tb_sqlx` read it.

## Decision

**A crate may only use concepts from crates it depends on.** If crate `A` does not depend on crate `B`, then no concept from `B` may appear in `A` — no type, name, string literal, or doc reference. The dependency graph is the *complete* statement of concept flow; a concept crossing a missing edge is **hidden coupling**.

**The domain is the extreme case.** `tb_domain` has no outgoing edges, so it is ignorant of its entire environment: no specific provider (Strava, intervals.icu), no database (Postgres), no web protocol (HTTP). It is the purest core *because* there is no edge for a concept to leak through.

**Crates meet through dependent-owned abstractions.** Where a higher crate must call into a lower one, the higher crate names the seam and the lower fills it (dependency inversion). `tb_domain` names the `Store` traits and owns `ActivityId`; `tb_sqlx` implements the store, `tb_strava` owns the `strava:` prefix. The dependent crate never names what the other crate is.

**Applied now:**

- `ActivityId` is an opaque `String` newtype. The `strava:` prefix — its constructor and raw-id parser — is private to `tb_strava`. `tb_exec::activity_url` just calls `tb_strava::strava_url`, with no provider discrimination: the redirect handler maps any error to a `"/"` redirect, so the "is this a Strava id?" check lives in the Strava driver, not the router.
- The provider-specific id tests move to `tb_strava`; the domain's id tests are opaque-string tests.

## Rejected alternatives

- **Let the domain know the id's prefix scheme** (the `#761` design: `ActivityId::strava` / `icu` / `strava_id` in the domain). Rejected: it couples the edgeless core to a specific provider and breaks the moment a second provider lands — the domain would have to relearn every provider's id format.
- **A public `strava_id()` / `strava()` API on `tb_strava`.** `tb_exec` no longer discriminates (it delegates to `strava_url`), so nothing outside `tb_strava` needs the prefix helpers — they stay private. A public API would be surface with no consumer.
- **Scrub only the domain.** The invariant is workspace-wide. A crate one edge up (`tb_exec`) that names a concept from a crate it does not depend on is the same hidden coupling. The rule holds at every edge, not just the leaf.

## Consequences

- `tb_domain` names no provider in its `ActivityId` handling — no constructors, no prefix parsing, and its id tests are opaque-string tests. The domain is now fully grep-clean of `strava`/`icu` literals: the snapshot fixture moved to `tb_sqlx`, `partid_get_by_source` was renamed to `source`, and the doc comments were scrubbed (#773) — the machine-checkable bar, in the spirit of ADR-0007 and ADR-0012.
- `ActivityId` construction and parsing live in `tb_strava` as private helpers; the future icu provider adds its own by implementing the domain's seams, without touching `tb_domain`.
- The snapshot move and the `partid_get_by_source` rename landed in #773, completing the domain's provider-ignorance.
- A contributor adding a provider or a crate is placed by the same rule: depend on what you name, name only what you depend on.
