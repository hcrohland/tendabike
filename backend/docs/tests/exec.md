# tb_exec tests

All `tb_exec` tests run in-memory (no database): the executor loop's control flow is structured as pure units testable with a channel and dummy futures (no store, no database), and the seams the loop drives are `mockall` witnesses, not hand-rolled stores. There are three of them: the **transaction** seam ([`MockConn`], `Txn`-only), the **domain** seam ([`MockDomainFacade`]), and the **provider lane** seam ([`MockLane`]). The loop tests live in `executor/executor_tests.rs` (the `#[cfg(test)]` module declared in `executor.rs`). The `Registry` lifecycle is split (spec #697): the lifecycle-core tests (spawn, `status`, `wake`, `stop_all`, the shop/user push no-op + delivery, the `push_to_recipient_shops` fan-out) live in `registry/core.rs`'s `#[cfg(test)] mod tests` (the core is driven **directly**, not through the facade). The one facade op with a store read — the `subscribe_shop` ownership gate and the shop-context delivery to an open shop stream — is verified against real Postgres in `tb_sqlx`'s `executor_seam.rs` (see [`sqlx.md`](sqlx.md)).

## Behavior ownership

`tb_exec` owns the **frame-push / orchestration logic**: the executor loop's control flow — message selection, the reclaim decision, the frame construction and routing, the backoff discipline, and the oneshot reply — driving the domain (the `ApiWrite`/`ApiRead` dispatch through the `DomainFacade` door) and the provider (the `ProviderLane`'s queue probe / gear / stats) inside transactions it owns.

- **What it mocks** — the **three seams the loop drives**:
  - the **transaction** — [`MockConn`] is a `mockall` `mock!` witness carrying **only** the `Txn` lifecycle (`commit`/`rollback`), the `TxnSource::Conn` bound: the loop opens it, works in it, and closes it. It no longer carries the `Store`/`StravaStore` surface.
  - the **domain** — [`MockDomainFacade`] is the `mockall` `#[automock]` witness for the `DomainFacade` door (the `exec`/`read`/`part`/`part_owner` dispatches). The domain logic runs for real behind the concrete `DomainFacadeImpl`; the tests swap in the mock.
  - the **provider lane** — [`MockLane`] is a `mockall` `mock!` witness for the `ProviderLane` seam (`probe`/`process`/`insert_sync`/`update_gear`/`all_stats`). **Strava** is stubbed (canned) through it for the gear-hydration / stats paths.

  The per-request tests assert on canned outcomes from these mocks — they verify the executor's behavior (how it drives each seam: `begin` → work → `commit`/`rollback`, and what it does with the results), not the seams'.
- **What it delegates to the seam** — the **store I/O** (real Postgres) lives in `tb_sqlx`'s `executor_seam.rs` (see [`sqlx.md`](sqlx.md)); the **domain-level results** live in the `tb_domain` suite (see [`domain.md`](domain.md)); the **Strava JSON → Part mapping** lives in `tb_strava` (see [`strava.md`](strava.md)).

## Coverage

- **Reclaim decision** (`next_action`): prefer a pending write over everything, a ready Strava event when no write, reap with no streams, sleep while streams listen
- **Message selection** (`select_message`): an API write wins over a ready Strava event, a Strava event when no write is pending, idle when the queue is empty and no write, idle when the channel is closed, a Strava error is its own state, limited when a stop is active, a read wins over a ready Strava event, a wake fired before / while parked
- **Housekeeping** (`stop_remaining`): the queue probe reaps queued events past the deadline; an expired deadline reaps nothing
- **Backoff** (`DbBackoff`): doubles until the cap, resets after a success
- **Frame routing** (`push_frame` / `Core::deliver`): non-empty and tombstone-only summaries delivered, empty summaries skipped (the pusher's guard), the `DeliveryContext` carries the acting user, the session's shop, and the resolved part owner; the complete routing policy lives in `Core::deliver` (user context to the personal stream, shop context to the shop stream / the part owner / not the shop owner (except when the part owner), the shops-the-operation-touched fan-out, the cross-recipient notification routing, the recipient shop-stream fan-out, the shop-context personal-stream catch-up)
- **Per-request** (`run_request`): a read arriving during a sleep is taken immediately, a failed read never arms the backoff and the loop survives, a committed read resets the backoff, `all_stats` round-trips through the loop transaction, gear hydration delivers the created-parts frame, a failed gear arms the backoff like a write, gear hydration without new gear delivers an empty summary, a shop-context write delivers the shop delivery context, a personal write delivers the personal delivery context
- **Registry lifecycle core** (`registry/core.rs`): the executor spawns on first demand and is `Running`, `stop_all` tears it down and the next demand respawns it, `wake` is a pure signal that never spawns, `push_to_shop` delivers to an open shop stream / is a no-op without a subscriber, `push_to_user` delivers to a running executor / is a no-op for a reaped one, the `deliver` routing policy (the shops-the-operation-touched fan-out, the cross-recipient notification routing, the recipient shop-stream fan-out, the shop-context personal-stream catch-up)
- **`subscribe_shop` ownership gate + shop-stream delivery** (`tb_sqlx`'s `executor_seam.rs`): `subscribe_shop` rejects a non-owner (`Forbidden` from the live ownership read), the owner receives a stream receiver, and a shop-context `deliver` reaches the open shop stream

## Key infrastructure

All in `executor/executor_tests.rs` (the `#[cfg(test)]` module declared in `executor.rs`); the doubles and fixtures are widened to `pub(crate)` and imported (not copied) by the `registry/core.rs` tests.

**The seams (what the loop drives):**

- **`MockConn`** — a `mockall` `mock!` witness (`#[cfg(test)]`) carrying **only** the `Txn` lifecycle (`commit`/`rollback`) — the `TxnSource::Conn` bound. The domain and provider work no longer rides it: it is just the transaction the loop opens, works in, and closes. `#[async_trait]` on the `impl Txn` block is what makes the `async fn` methods desugar (see [`docs/research/mockall-conn-witness.md`](../../../docs/research/mockall-conn-witness.md) §1).
- **`MockDomainFacade`** — the `mockall` `#[automock]` witness for the `DomainFacade` door (the `exec`/`read`/`part`/`part_owner` dispatches), a crate-internal `cfg(test)` double generated on the `DomainFacade` trait in `facade.rs`. Generic over the store type `C`; the tests instantiate it over `MockConn` (the loop's transaction). Set expectations with the flat `expect_exec` / `expect_read` / `expect_part` / `expect_part_owner` API.
- **`MockLane`** — a `mockall` `mock!` witness for the `ProviderLane` seam (`probe`/`process`/`insert_sync`/`update_gear`/`all_stats`): the executor's door into the provider ops. Its `Store` is `MockConn` (the `FakeSource`'s connection), so the loop's transaction ties to it.

**The `TxnSource` / lane / pusher test doubles:**

- **`FakeSource`** — a `TxnSource` whose `begin` hands out the test's configured `MockConn` (one per loop-owned transaction) or fails (the DB-down corner).
- **`RecordingPusher`** — a `FramePusher` test double: records the `deliver` calls (the `DeliveryContext` and the `Summary`) so the loop's delivery intent is observable. `Clone` shares the recording (`Arc`), so a test keeps an observation handle after moving the pusher into the context.
- **`test_context`** — the loop's wiring and state (`LoopContext`), built with a `FakeSource`, a `MockDomainFacade<MockConn>`, a `MockLane`, a frame channel, and a `RecordingPusher`; the per-request tests (`run_request`) drive one message through the real loop.

**The dummy futures and fixtures:**

- **`ready_probe` / `Never`** — the canned queue-probe futures the `select_message` / `next_action` units test against (immediately-ready vs. never-ready).
- **fixtures** — `request(...)` (a `Request` bundling a write/read and its oneshot reply) and the canned domain values (`part_owned_by`, `summary_owned_by`, `summary_with_service`, `non_empty_summary`, `notification_part_registered`, `part_test_bike`, `user_record`). The set grows — the test module in `executor/executor_tests.rs` is the source of truth, so read it for the current fixtures.

## Writing new tests

```rust
use super::{test_context, run_request};
use crate::{FakeSource, MockConn, MockDomainFacade, MockLane, RecordingPusher};

#[tokio::test]
async fn my_test() -> TbResult<()> {
    // The transaction: a `Txn`-only `MockConn` the loop opens, works in, and
    // commits.
    let mut conn = MockConn::new();
    conn.expect_commit().returning(|| Ok(()));
    let source = FakeSource::with(conn);
    // The domain door: the dispatch is canned — the domain logic does not run
    // behind the mock.
    let mut facade = MockDomainFacade::new();
    facade.expect_read().returning(|_r, _s, _st| {
        Box::pin(async { Ok(ReadOutcome::UserRecord(/* a canned User */)) })
    });
    let ctx = test_context(
        source,
        facade,
        MockLane::new(),
        broadcast::channel(4).0,
        Box::new(RecordingPusher::new()),
    );
    // ... drive a `Request` through `run_request`; assert on the frame the
    // `RecordingPusher` recorded and the oneshot reply.
    Ok(())
}
```

- `#[tokio::test]` for async; return `TbResult<()>` so `?` propagates domain errors with context.
- Script only the seam methods the operation under test calls — the `Txn` (`commit`/`rollback`) on `MockConn`, the dispatches on `MockDomainFacade`, the provider ops on `MockLane`. Each mock has no state: an unscripted call panics, which is a test bug, not a seam bug.
- The pure control-flow units (`next_action`, `select_message`, `stop_remaining`, `DbBackoff`) take a channel and dummy futures (`ready_probe` / `Never`) — no `MockConn` at all.
- Canned domain values come from the fixtures (`summary_owned_by`, `non_empty_summary`, …); the suites have no wall clock, so every fixture time is a fixed value.

## Gotchas

- **The seams are canned, the logic is real.** The per-request tests (`run_request`) assert on canned outcomes from the `MockConn` / `MockDomainFacade` / `MockLane` mocks: they verify the executor's behavior — how it drives the seams (`begin` → work → `commit`/`rollback`) and what it does with the results (frame routing, the backoff discipline, the oneshot reply) — not the seams' internal behavior. The domain logic runs for real behind the concrete `DomainFacadeImpl` in production, and the store's behavior (and the domain logic that runs over it) is verified against real Postgres in `tb_sqlx`'s seam suites (see [`sqlx.md`](sqlx.md)).
- **`mockall` is a `tb_exec` dev-dependency for the crate's own suite** — the `MockConn`, `MockLane`, and `MockDomainFacade` witnesses are crate-internal `cfg(test)` doubles; no production `pub` item depends on them. The `test-support` feature is what a *dependent's* test suite enables: it exists solely for the web layer's contract suite and gates the `ServiceApi` / `ProviderService` automocks (`api.rs`).
- **`#[async_trait]` on each `impl` block** is what makes the `async fn` methods desugar in the generated `mock!` code — a plain `impl` on the async methods does not compile.
- The loop computes no domain state of its own: it drives the `DomainFacade` dispatch and the `ProviderLane` ops inside transactions it owns, so a test that wants a domain result asserts on the frame the loop pushes, not on a store row.
