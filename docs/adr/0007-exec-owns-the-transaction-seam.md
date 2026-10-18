# ADR-0007: exec owns the transaction seam

## Status

Accepted. The design is locked and execution is in progress (step 1 of 4, spec §9).

## Context

ADR-0005 introduced the per-user executor loop in `tb_exec`, but the transaction seam stayed split: the loop opened the executor's transactions, while the presentation layer (`tb_axum`) opened its own — every read route, the Strava plumbing (login, webhook, sync, deauthorise), and the admin routes ran inline `begin` … `commit` in the handlers. Three consequences:

- the web layer named the seam types (`TxnSource`, `Txn`, `SqlxConn`, `DbPool` — 116 references in 14 files, spec §1), so a second presentation driver (a CLI) would have to learn the same seam vocabulary;
- reads bypassed the executor's per-user FIFO order entirely — a read could race an in-flight write on the same user;
- the transaction lifecycle was decided in two places (the loop and the handlers), with the same begin/commit/rollback discipline duplicated.

This ADR records the design for moving the transaction seam into `tb_exec` so that only the executor loop opens transactions. The full executable spec is issue [#558](https://github.com/hcrohland/tendabike/issues/558); the decisions it synthesizes are the seven tickets of the ["exec owns the transaction seam — axum as a dumb interface" map](https://github.com/hcrohland/tendabike/issues/549) ([#550](https://github.com/hcrohland/tendabike/issues/550)–[#556](https://github.com/hcrohland/tendabike/issues/556)).

## Decision

**`tb_exec` behaviorally owns the transaction seam.** Only the executor loop opens transactions:

- **Reads become executor messages.** A new `ApiRead` descriptor enum in `tb_domain` (one variant per per-user read route, carrying exactly the parsed arguments the handler extracts today) is enqueued on the user's per-user request channel beside the writes; the loop runs each read in its own transaction (read-only within it, by spec) and resolves a sealed `ReadOutcome` enum over the read payloads. A read inherits the lane's discipline: FIFO behind every earlier message, never delayed by the Strava `Stop` nor the `DbBackoff`, and a failed read never arms the backoff.
- **The non-message ops become direct `Registry` methods.** The ops that are not per-user messages (rescan, the admin syncs, the Strava plumbing — login upsert, webhook accept, the redirect URL reads, deauthorise/delete) become direct `Registry` methods that own their own transaction(s) and any lifecycle control; the presentation layer calls only the service API.
- **The `Registry` moves to `tb_exec`, owning the `TxnSource`** (injected at construction; the seam traits stay `pub` because `tb_sqlx` implements them). The exec-owned surface is a trait — **`ServiceApi`** — that is the app's presentation-agnostic service API: axum (and the named second driver, a CLI) depend on the trait and never name a seam type.
- **`tb_axum` becomes a zero-reference dumb presentation layer**: no `TxnSource`/`Txn`/`SqlxConn`/`DbPool` anywhere in its source — the grep is the test. Handlers thin to one shape: extract the session and the JSON/path arguments, call the `ServiceApi`, map the outcome.

**The HTTP contract is unchanged** — routes, status codes, response bodies. The only intentional behavior deltas are the all-users sync's per-user wake and the `update_gear` hydration frame; both are internal.

**Single node.** One process owns a user's executor and their SSE streams (ADR-0005). Durable state lives in Postgres; the in-memory structures are per-process and respawn on demand.

The domain-flow claim is unchanged and generalized: the domain computes entity state; triggers drive domain operations. The executor computes nothing of its own — now for reads too.

## Rejected alternatives

- **Keep the handler-owned transactions (the status quo).** Two places decide the transaction lifecycle, reads bypass the per-user FIFO order, and the web layer must keep naming the seam types — a second driver inherits the same coupling.
- **Capability closures for reads (the handler hands the loop a closure to run).** The service surface a second driver gets must be explicit: an enum *is* that API, and the read's behavior (which domain op, which arguments) lives in the domain once, not in the presentation layer. Closures are uncomparable, unloggable, and break the loop's message discipline.
- **A plain (non-transactional) connection path on the seam for reads.** The measured cost was re-implementing 92 store/Strava impl bodies (or a one-time mechanical refactor of all of them) for two saved round-trips and a type guarantee the contract cannot observe. The seam keeps its current shape (nine store sub-traits + `Txn`/`TxnSource`).
- **Keep the `Registry` in `tb_axum`.** The Registry is the service API's single handle (its ordered surface is the per-user channel; its direct surface is the non-message ops); leaving it in the web layer keeps the web layer the seam's owner, which is the disease.

## Consequences

- The dependency graph layers (each layer depends only on the layers below it): `tb_axum → tb_domain, tb_strava, tb_exec` (naming `ServiceApi`, `ExecutorStatus`, and the cadence constants — never a seam type); `tb_exec → tb_domain, tb_strava` (gains the `Registry`, `ServiceApi`, the read dispatch, the direct ops); `tb_sqlx → tb_domain, tb_strava, tb_exec` (implements `Txn`/`TxnSource` — unchanged); `tb_strava` gains the executor's concrete `StravaUserSession`; `tb_domain` gains the read side (`ApiRead`, `ReadOutcome`, the `apiread` dispatch beside `apiwrite::exec`, the unified `UserState` composite).
- `AppState` goes non-generic (`Arc<dyn ServiceApi>`); `start`/`routes` and every handler drop their `TxnSource` bound and the `source` parameter.
- The axum suite's `TxnSource` doubles are deleted in favor of a recording `StubApi`; the contract tests assert the recorded messages — the suite gains the ability to check what a handler sent.
- The migration is four steps, one PR each, every step green (spec §9); the zero-reference grep lands last as a one-time acceptance check, not a CI gate.
