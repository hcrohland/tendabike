# ADR-0007: The executor owns the transaction seam

## Status

Accepted.

## Context

ADR-0005 moved the write path through a per-user executor loop in `tb_exec`, but left the seam's *ownership* in the web layer. `tb_axum` held the executor `Registry`, was generic over the `TxnSource` (an `AppState<S: TxnSource>`), and every read route (~16) plus the Strava plumbing routes (login upsert, webhook accept, admin sync, rescan, the redirect URL reads, deauth/delete) opened transactions inline in handlers. The seam types (`TxnSource`, `Txn`, `SqlxConn`, `DbPool`) were named throughout `tb_axum` source — the presentation layer knew everything about the store it was supposed to be insulated from. The surface a presentation layer needs (run a read, enqueue a write, subscribe to frames, run the plumbing ops) was entangled with the storage it drives, and a second presentation layer — the named second driver, a CLI — could not be written without re-learning the seam.

## Decision

**`tb_exec` behaviorally owns the transaction seam; `tb_axum` is a dumb interface among potentially many presentation layers.** Only the executor loop opens per-user transactions. Reads cross into the executor as descriptor messages (`ApiRead` → sealed `ReadOutcome`, run in a loop-owned transaction, read-only by spec). The non-message ops (rescan, admin sync, Strava plumbing) become direct `Registry` methods that own their own transactions. The `Registry` moves to `tb_exec` and owns the `TxnSource`; the executor's session becomes a concrete `tb_strava::StravaUserSession`.

The exec-owned surface is exposed as a trait — **`ServiceApi`** in `tb_exec` — that is the app's presentation-agnostic **service API**: axum (and the future CLI) depend on the trait through `Arc<dyn ServiceApi>` and never name a seam type. The acceptance bar is grep-verifiable: **zero references to `TxnSource`/`Txn`/`SqlxConn`/`DbPool` anywhere in `tb_axum` source** (prod, tests, test_support). The traits stay `pub` in `tb_exec` — `tb_sqlx` must implement them; axum simply never names them.

The full executable spec is issue [#558](https://github.com/hcrohland/tendabike/issues/558); the decisions it synthesizes are the seven tickets of the "exec owns the transaction seam — axum as a dumb interface" map ([#549](https://github.com/hcrohland/tendabike/issues/549), tickets [#550](https://github.com/hcrohland/tendabike/issues/550)–[#556](https://github.com/hcrohland/tendabike/issues/556)).

## Rejected alternatives

- **Keep reads inline in axum on plain (non-transactional) connections.** The seam would need a plain-connection path: re-implementing ~92 store/Strava impl bodies (or a one-time mechanical refactor of all of them) for two saved round-trips and a type guarantee the contract cannot observe. Reads instead ride the loop's transaction.
- **Capability-closure reads** (a `Box<dyn FnOnce(&mut impl Store) -> TbResult<R> + Send + 'static` the loop runs). Closures are uncomparable, unloggable, and put the read's behavior in the presentation layer; an explicit descriptor enum *is* the service API a second driver gets, and the read's behavior lives in the domain once.
- **A concrete `Registry` as the surface (no trait).** One adapter means a hypothetical seam. The trait exists because two adapters do — the production `Registry` and the test `StubApi` — and because the CLI takes the same `Arc<dyn ServiceApi>` handle.
- **A separate read channel (a non-gated lane).** A second channel would break the per-user FIFO total order — the ordering contract a second driver relies on.

## Consequences

- Crate graph: `tb_axum` loses its `TxnSource` genericity (`AppState` holds `Arc<dyn ServiceApi>`); the composition root (`app`) builds the `Registry` over the `DbPool`. `tb_domain` gains the read side (`ApiRead`, `ReadOutcome`, the dispatch, the user-composite) bounded by `Store`; `tb_strava` gains `StravaUserSession`.
- The in-memory seam doubles (`LiveSource`/`LiveConn`/`FakeSource`/`FakeConn`) are deleted, not moved; the axum suite becomes pure HTTP-contract tests against a recording `StubApi` (no loop, no in-memory store).
- The both-stores contract governs the test split: Postgres is the source of truth for database behavior; the in-memory store follows at the call level only.
- The HTTP contract (routes, status codes, response bodies) is unchanged. The only intentional behavior deltas are the all-users sync's per-user wake and the `update_gear` hydration frame — both internal.
- The single-node assumption stands (ADR-0005); multi-node is out of scope.
