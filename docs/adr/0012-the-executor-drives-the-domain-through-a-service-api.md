# ADR-0012: The executor drives the domain through a service API

## Status

Accepted. The current-state record of the write path and its seams. It consolidates four earlier ADRs (0005, 0007, 0010, 0011), now in [`archive/`](archive/). The human-readable tour of the crate lives in [`backend/docs/exec.md`](../../backend/docs/exec.md).

## Context

A mutation reaches the domain from a presentation layer (today the web layer `tb_axum`; a CLI is a named second driver). The design has to answer four questions: what drives the domain, who owns the transaction, how the Strava event lane is typed, and who may touch a store. Each was settled in turn, and each refined the last. This ADR records the end state so a reader does not have to reconcile four versions.

## Decision

**A per-user executor loop drives the domain, over SSE.** `tb_exec` holds one long-lived task per user — the executor loop — that consumes two message sources and runs each in one transaction: API writes (an in-memory channel) and Strava events (the `strava_events` database queue). It pushes each completed message's `Summary` to the client over a session-scoped SSE stream (`GET /api/user/stream`). The old 60-second poll and `GET /strava/hooks` are gone — SSE is the only client lane, and a dropped stream re-establishes and re-hydrates via a full `GET /api/user/summary`. The design assumes a single node (one process owns a user's executor and its streams); multi-node is out of scope.

**The executor owns the transaction seam; the web layer is a dumb interface.** Only the executor loop opens per-user transactions. Reads cross into the executor as descriptor messages (`ApiRead` → a sealed `ReadOutcome`, run in a loop-owned, read-only transaction). The non-message ops (rescan, admin sync, the Strava plumbing) are direct `Registry` methods that own their own transactions. The `Registry` lives in `tb_exec` and owns the `TxnSource`. The surface a presentation layer needs is a trait — **`ServiceApi`** in `tb_exec` — and `tb_axum` (and the future CLI) depend on it through `Arc<dyn ServiceApi>` and never name a seam type. The acceptance bar is grep-verifiable: **zero references to `TxnSource` / `Txn` / `SqlxConn` / `DbPool` anywhere in `tb_axum` source** (prod, tests, test_support).

**The Strava seam takes a session only where it receives one.** The Strava driver sits on top of the domain and is driven from two kinds of context. The API-write lane and the `Registry`'s direct ops carry real acting sessions (behavior-bearing: the write lane's active shop, the admin gates), so their entry points take a `Session`. The event lane (the executor draining `strava_events`) has no caller and no context — only the user id — so its entry points take a `UserId`. Where the id-only chain crosses into a session-typed call it constructs `OnBehalf`, a private minimal `Session` impl that encodes the event lane's actual context (`shop() = None`, `is_admin() = false`). The domain interface is unchanged: domain operations take the acting session; the session is the domain's language, and the Strava driver speaks it only where it has one.

**The entity owns its store.** The domain is storage-agnostic: every operation goes through the store traits, and only the entity that owns a store may access it. A store method (`<entity>_<op>`) is called only from its owning entity's code; a cross-entity need is served by the callee exposing the operation it owns and the caller invoking that entity method — never the callee's store. This applies to every store, including `StravaDataStore`. Exempted: the sqlx store files (they *are* the store), the `MockStore` double, the test and seam suites, and `build_snapshot.rs`. The rule is machine-checked by [`.githooks/check-store-access.py`](../../.githooks/check-store-access.py) (run by the `pre-commit` hook).

## Rejected alternatives

*Executor and SSE*

- **Keep the poll (with or without a feature flag).** The 60-second ceiling is the motivation for the effort; keeping it preserves the ceiling and a second write path that bypasses the executor's per-user serialization.
- **Put the loop in `tb_axum` or `tb_domain`.** `tb_axum` is the web layer; a long-lived per-user task with a transaction lifecycle is not presentation. `tb_domain` cannot host the transaction seam without a cycle (the in-memory store must not implement it). A dedicated `tb_exec` crate keeps the loop and its seam in one place.
- **A supervisor that restarts a panicked executor.** The client's reconnect + full refresh is the recovery path; a supervisor would hide the visible alarm (the persistent spinner) a poison event produces.

*The transaction seam*

- **Keep reads inline in `tb_axum` on plain connections.** The seam would need a plain-connection path: re-implementing the store impl bodies for two saved round-trips. Reads instead ride the loop's transaction.
- **Capability-closure reads** (a boxed closure the loop runs). Closures are uncomparable and unloggable, and put the read's behavior in the presentation layer; an explicit descriptor enum *is* the service API a second driver gets.
- **A concrete `Registry` as the surface (no trait).** The trait exists because two adapters do (the production `Registry` and the test `StubApi`) and because the CLI takes the same `Arc<dyn ServiceApi>` handle.
- **A separate read channel.** A second channel would break the per-user FIFO total order the contract relies on.

*The Strava seam*

- **Uniform session at the seam.** Forces the id-only event lane to construct a fake session at the call site; the parameter type lies about the information the caller has.
- **`UserId` everywhere in the Strava driver.** Loses behavior-bearing context: the write lane's active shop no longer reaches the shop-scoped `Part::create`, and the admin gates have nothing to read.
- **Relax the domain to take ids.** The acting session is the domain's concept of "on whose behalf"; the shop-scoped and admin-gated ops need it.

*Store ownership*

- **Exempt an aggregate root.** Let `User` cascade-delete directly. Rejected: it carves a named hole in an otherwise-uniform rule; the bulk-by-collection `delete_many` methods are small and reuse the existing store calls.
- **A domain-service composition point.** A use-case function allowed to touch all stores. Rejected: it reintroduces the "a non-entity layer reaches across stores" shape the rule removes.
- **`delete_all_for_user` per entity.** Each entity finds-and-deletes its own rows for a user. Rejected: it duplicates the summary read and scatters the FK-ordering logic.

## Consequences

- **Crate graph:** `tb_exec` holds the loop, the `Registry`, the `Txn` / `TxnSource` / `ServiceApi` traits, and the `Message` enum. `tb_axum` depends on `tb_exec` (the `ServiceApi` trait) but not on `tb_sqlx` — the concrete `DbPool` is injected at the composition root (`app`). `tb_exec` depends on the provider layer (`tb_provider`, which holds the strava module); `tb_sqlx` implements the seam traits.
- **The `Summary` is the id-keyed map** (a `null` entry is a tombstone); operations return it directly, and it is the payload of every SSE frame.
- **The web layer is presentation-only:** it extracts the session, enqueues the write, and answers a status (204; creates 201+entity; onboarding 200+User) — the change rides the stream.
- **Two machine-verifiable bars:** `tb_axum` names no seam type (grep), and no entity touches a foreign store (the pre-commit hook).
- The single-node assumption stands; multi-node is out of scope.
- The detailed mechanics — the loop's control flow, the `Registry`'s lifecycle, the frame-push boundary — are in [`backend/docs/exec.md`](../../backend/docs/exec.md).
