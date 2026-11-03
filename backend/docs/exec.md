# tb_exec — the per-user executor

`tb_exec` is the app's service layer. It holds one long-lived task per user —
the **executor loop** — that consumes the user's work, runs each piece in one
transaction, and pushes the result to the client's SSE streams. It also holds
the **Registry**, the manager that decides when an executor exists, and the
**`ServiceApi`** trait, the presentation-agnostic surface the web layer (and
any future driver, e.g. the CLI) calls.

The crate computes no domain state of its own: it drives the existing
`tb_domain` operations and `tb_strava` event processing inside transactions
it owns.

The design assumes a **single node**: one process owns a user's executor and
their streams. Multi-node delivery is out of scope.

## The two halves

```text
tb_axum (web layer) — or a future CLI
   │  Arc<dyn ServiceApi>
   ▼
Registry ──────────────────────────────  the manager
   │  owns the TxnSource
   │  spawns / reaps / tracks
   ▼
LoopContext::run ──────────────────────  the worker (one task per user)
   │  one transaction per message
   │  pusher.deliver(context, summary)
   ▼
Core::deliver ──▶ SSE frame channels (personal + shop streams)
```

### The executor loop (`executor.rs`)

One task per user, spawned on demand (the user's first SSE connect or API
write) and reaped when idle with no stream attached; the next demand
respawns it. An attached stream keeps it alive — the idle window is then just
the cadence at which the loop re-probes the Strava queue.

**Inputs** — two message sources, checked in priority order (a client action
never waits):

1. The user's in-memory **request channel** — API writes, reads, the gear
   hydration write, and the admin stats read. Requests process in **FIFO
   total order per user**: a read enqueued after a write sees that write.
   This ordering contract is what a second driver gets.
2. The **`strava_events` database queue** — probed each cycle. A **wake**
   signal (fired by the Registry when the webhook queues an event)
   interrupts the idle wait so the queue is re-probed at once. The wake is a
   signal only; the queue is the source of truth, so a reaped executor loses
   nothing.

**One transaction per message** — the loop's core discipline:
`begin` → dispatch → `commit` on success, `rollback` on any failure.

- An API write dispatches through the domain's `exec(ApiWrite, …)` with the
  request's acting session.
- A Strava event drives `tb_strava`'s `process` with the user's id — the
  event lane has no acting session (the queue is per-user, the data is the
  user's own, no shop applies, per [ADR-0009](../../docs/adr/0009-the-active-shop-is-per-request-not-per-browser.md)).

**Delivery** — after a successful commit, the resulting `Summary` goes
through the `FramePusher` seam: the loop emits a delivery intent — a
`DeliveryContext` (acting user, the session's shop, the resolved part owner)
plus the `Summary` — and the pusher owns the complete routing policy (which
streams the frame reaches: the personal stream, shop streams, the
cross-recipient notification fan-out). The pusher is a no-op for an empty
`Summary`.

**Failure discipline** — message failures are recoverable; only a failed
`rollback` ends the loop:

- A failed API write **fails fast**: the oneshot reply carries the error,
  the message is consumed (the client is the retry layer), the loop continues.
- A **DB failure** (failed `begin`/`commit`/probe) pauses the loop on an
  in-memory exponential backoff (1s → 30s cap); any committed transaction
  resets it.
- A **Strava failure** is classified inside `process` (rate-limit `Stop` row,
  auth disable, event delete for permanent failures).

There is deliberately no supervisor: the client's reconnect + full
`GET /api/user/summary` refresh is the recovery path, and a dead executor
must stay visible (the persistent spinner), not hidden.

### The Registry (`registry.rs`, `registry/core.rs`)

The Registry has two layers:

- **The facade** (`Registry`) owns the `TxnSource` (injected at
  construction; the loop's source is a clone of it) and implements
  `ServiceApi` and `FramePusher`, delegating the lifecycle to the core.
- **The core** (`Core`, crate-internal) is the "how does an executor stay
  alive" machinery: the per-user executor map and the per-shop frame-channel
  map, the spawn/reap/respawn logic, the wake signal, the stop-all teardown,
  the shop-stream lifecycle, and the frame-routing policy (`Core::deliver`).

Each executor's **settled fate** is shared with its streams through a
`watch` channel as an `ExecutorStatus`: `Running`, `Reaped` (a clean idle
exit — the stream keeps its connection, heartbeats, re-subscribes), or
`Dead` (a lifecycle error or panic — the stream ends so the client
reconnects and respawns).

**`ServiceApi`** is the surface a presentation layer calls — 15 methods in
two groups:

- **Channel ops** (ride the per-user FIFO): `read`, `write`,
  `hydrate_gear`, `subscribe`, `all_stats`.
- **Direct ops** (own their own transactions): `subscribe_shop`, `rescan`,
  `sync`, `trigger_processing`, `login_from_token`, `accept_event`,
  `gear_url`, `activity_url`, `user_url`, `deauthorize`, `delete_user`.

The inclusion rule is *only what a presentation layer calls*: lifecycle
internals (`wake`, `stop_all`, …) stay off the trait.

## The seams

Three traits make the crate pluggable, and each is a real seam (more than
one adapter):

| Seam | Role | Adapters |
|---|---|---|
| `TxnSource` / `Txn` | The transaction lifecycle the loop drives: `begin` → work → `commit`/`rollback`. The loop is generic over it and names no concrete store. | `DbPool` (in `tb_sqlx`), test doubles in the seam suite |
| `FramePusher` | One method, `deliver(context, summary)`: route a `Summary` to its delivery targets given the acting session's context. The implementation owns the complete routing policy. | The `Registry` (production), recording/no-op doubles in tests |
| `ServiceApi` | The presentation-agnostic service API (above). | The `Registry` (production), the recording stub in the web layer's suite |

`DetachedBegin` wraps a `TxnSource` with a cancellation-safe `begin`: the
`BEGIN` runs to completion in a spawned task, so a cancelled caller (a
dropped `select!` branch, a disconnected client) never leaves a half-open
transaction that would poison the pool. It lives here — not in `tb_sqlx` —
because managing the pool's lifecycle is an executor-level concern.

## Where the pieces sit

| Piece | Crate |
|---|---|
| Loop, `Registry`, `ServiceApi`, the seams | `tb_exec` |
| The Postgres half of `TxnSource`/`Txn` | `tb_sqlx` (depends on `tb_exec`) |
| The domain ops the loop drives (`exec`, `ApiRead`, `Summary`) | `tb_domain` |
| Strava event processing (`process`, the queue) | `tb_strava` |
| The web layer (routes, SSE) | `tb_axum` (depends on `tb_exec`, **never** `tb_sqlx`) |
| The composition root (builds the `Registry` over the `DbPool`) | `app` |

Dependency direction: `tb_exec → tb_domain + tb_strava`; `tb_sqlx →
tb_exec`; `tb_axum → tb_exec`.

## History

Two ADRs shaped it:

- **[ADR-0005](../../docs/adr/0005-message-based-executor-loop-and-sse-push.md)**
  replaced the client's 60-second Strava poll with a server-side,
  message-based loop that pushes `Summary` frames over SSE. The loop is the
  only driver of the per-user transaction lifecycle.
- **[ADR-0007](../../docs/adr/0007-exec-owns-the-transaction-seam.md)**
  moved the transaction seam out of the web layer: `tb_axum` became a dumb
  interface that holds `Arc<dyn ServiceApi>` and names no seam type
  (`TxnSource`/`Txn`/`DbPool` appear nowhere in its source — a
  grep-verifiable bar). A second presentation layer can now be written
  without re-learning the storage.

## Going deeper

- The **module docs** in the source are the fine-grained spec:
  `src/executor.rs` (the loop, `LoopContext`, failure semantics),
  `src/registry.rs` (the facade, `ServiceApi`, `FramePusher`),
  `src/registry/core.rs` (the lifecycle core, the routing policy),
  `src/message.rs` (the request channel, the frame sink), `src/txn.rs`
  (the transaction seam, `DetachedBegin`).
- **Testing**: `docs/tests/exec.md` — the in-memory loop units, the
  `mockall` witness, and the frame-push ownership boundary. The
  transaction integration is verified against real Postgres in
  `tb_sqlx`'s `tests/executor_seam.rs`.
- **Decisions**: ADR-0005 (the loop and SSE push), ADR-0007 (exec owns the
  seam), ADR-0009 (the active shop is per request), ADR-0011 (the Strava
  seam takes a session only where it receives one).
