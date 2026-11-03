# ADRs — Index

Accepted decisions for Tendabike, in reading order. Each records a decision that is true **now**; together they describe the current architecture. For the history of how a decision was reached — and for decisions that no longer stand — see [`archive/`](archive/).

For the mechanics, read [`backend/docs/exec.md`](../backend/docs/exec.md) (the executor crate) and [`docs/agents/domain-flow.md`](../agents/domain-flow.md) (how the client stays in sync). Terminology is in [`GLOSSARY.md`](../GLOSSARY.md).

## Live

| ADR | Decision |
|-----|----------|
| [0012 — The executor drives the domain through a service API](0012-the-executor-drives-the-domain-through-a-service-api.md) | The write path: a per-user executor loop over SSE owns the transaction seam (`ServiceApi`; the web layer is a dumb interface); the Strava seam takes a session only where it has one; the entity owns its store. Read this first. |
| [0002 — Plan due-ness is a client-side derivative](0002-plan-due-ness-is-a-client-side-derivative.md) | Due-ness is a client-side derivative computed against the clock, not a stored server value. |
| [0009 — The active shop is per-request, not per-browser](0009-the-active-shop-is-per-request-not-per-browser.md) | The acting session's context (the active shop) is per-request, carried in a header/query, not persisted per-browser. |
| [0008 — Guards live where the invariant lives](0008-guards-live-where-the-invariant-lives.md) | A guard is placed at the layer that owns the invariant it enforces. |
| [0003 — Flat attachment rows against the top-level gear](0003-flat-attachment-rows-against-the-top-level-gear.md) | The attachment stores one flat row per part (`gear` + `hook`); the mount tree is derived, not stored. |
| [0004 — The types collection is configuration, not state](0004-types-collection-is-configuration-not-state.md) | `types` is a plain, non-reactive map carrying the type vocabulary, loaded once. |

## Archive

Superseded and rejected decisions, kept for the record and out of the default reading path:

- [0001 — Plan status rule placement](archive/0001-plan-status-rule-placement.md) — withdrawn; superseded by 0002.
- [0005 — The message-based executor loop and SSE push](archive/0005-message-based-executor-loop-and-sse-push.md) — superseded by 0012.
- [0006 — Test infrastructure generic over the domain](archive/0006-generic-over-domain-test-rearchitecture.md) — rejected.
- [0007 — The executor owns the transaction seam](archive/0007-exec-owns-the-transaction-seam.md) — superseded by 0012.
- [0010 — The entity owns its store](archive/0010-entity-owns-its-store.md) — superseded by 0012.
- [0011 — The Strava seam takes a session only where it receives one](archive/0011-the-strava-seam-takes-a-session-only-where-it-receives-one.md) — superseded by 0012.
