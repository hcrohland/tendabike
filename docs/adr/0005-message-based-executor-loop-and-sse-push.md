# ADR-0005: The message-based executor loop and SSE push

## Status

Accepted.

## Context

TendaBike keeps the client in sync with the backend through three lanes (`docs/agents/domain-flow.md`): hydration, writes, and the Strava drain. The Strava lane is a 60-second poll: the client polls `GET /strava/hooks`, which drains the user's next queued Strava event and returns the `Summary` of what changed. The poll has a 60-second latency ceiling, and the drain is a one-shot, client-triggered operation — the server has no continuous view of what to do next for a user.

This ADR records the design for replacing that poll with a message-based executor loop that drives the domain, and the client contract that rides on it. The full executable spec is `docs/specs/executor-loop.md`; the decisions it synthesizes are the five tickets of the "Message-based executor loop for the domain — design" map ([#438](https://github.com/hcrohland/tendabike/issues/438)–[#442](https://github.com/hcrohland/tendabike/issues/442)).

## Decision

**A per-user executor loop drives the domain.** A new crate, `tb_exec`, holds a per-user executor: a long-lived task that consumes two message sources — API writes (an in-memory channel) and Strava events (the existing `strava_events` DB queue) — runs each in one transaction, and pushes the resulting `Summary` to the client over SSE. The loop is the only driver of the transaction lifecycle for the executor; the `Store` marker trait stays method-free, and a new `Txn` trait (with a `TxnSource` companion) in `tb_exec` names the lifecycle the loop drives.

**Placement.** The loop, the `Txn`/`TxnSource` traits, and the `Message` enum live in `tb_exec`. The Strava `Event` and its `process` stay in `tb_strava` (the dependency direction is `domain ← strava`; the `Event` never enters `tb_domain`). `tb_sqlx` implements `Txn`/`TxnSource` for `SqlxConn`/`DbPool` (a new `tb_sqlx → tb_exec` dependency). `tb_axum` wires the loop, the SSE stream, and the write handlers. `tb_domain` gains only the `ApiWrite` enum and the `exec(ApiWrite, &mut impl Store)` dispatch.

**SSE replaces the poll.** The client opens a session-scoped `GET /api/user/stream` (SSE) and merges every frame (a `Summary` as an id-keyed map, `null` = tombstone). The 60-second poll and `GET /strava/hooks` are removed, not kept as a fallback. On a dropped stream the client re-establishes the stream, then issues a full `GET /api/user/summary` refresh to catch up. Writes return a status (204; creates 201+entity, onboarding 200+User) and the change rides the stream. The cutover is a hard one in a single release.

## Rejected alternatives

- **Keep the poll (with or without a feature flag).** The poll's 60-second ceiling is the motivation for the whole effort; keeping it as a fallback would preserve the ceiling and a second write path that bypasses the executor's per-user serialization.
- **Put the loop in `tb_axum` or `tb_domain`.** `tb_axum` is the web layer; a long-lived per-user task with a transaction lifecycle is domain-adjacent machinery, not presentation. `tb_domain` cannot host the `Txn` trait without a `tb_domain → tb_exec` cycle (the `MemStore` must not implement `Txn`). A new `tb_exec` crate keeps the loop and its seam in one place, with the `StravaStore` bound on the loop, not the domain.
- **A supervisor that restarts a panicked executor.** The client's reconnect + full refresh is the recovery path; a supervisor would hide the visible alarm (the persistent spinner) that a poison event produces.

## Consequences

- A new crate in the workspace; `tb_sqlx` and `tb_axum` depend on `tb_exec`; the `Store` marker doc gains the executor as a lifecycle driver.
- The `Summary` becomes the id-keyed map (the `Vec` struct is dropped; `SumHash` is renamed `Summary`); operations return the map directly.
- The client's `Header.svelte` drain loop and the 60-second poll are removed; the SSE listener replaces them.
- The design assumes a single node (one process owns a user's executor and their SSE stream); multi-node is out of scope.
