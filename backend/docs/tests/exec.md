# tb_exec tests

All `tb_exec` tests run in-memory (no database): the executor loop's control flow is structured as pure units testable with a channel and dummy futures (no store, no database), and the transaction seam is a `mockall` `mock!` witness, [`MockConn`], not a hand-rolled store. The loop tests live in `executor/executor_tests.rs` (the `#[cfg(test)]` module declared in `executor.rs`); the `Registry` stream-lifecycle tests live in `registry.rs`'s `#[cfg(test)] mod tests`.

## Behavior ownership

`tb_exec` owns the **frame-push / orchestration logic**: the executor loop's control flow — message selection, the reclaim decision, the frame construction and routing, the backoff discipline, and the oneshot reply — driving the domain (`ApiWrite` dispatch, `tb_strava` event processing) inside transactions it owns.

- **What it mocks** — the **store trait seam**: [`MockConn`] is a `mockall` `mock!` witness accumulating the full `Store` surface (the nine sub-traits + the method-free `Store` marker), the `StravaStore` surface, and the `Txn` lifecycle (the `TxnSource::Conn` bound). The per-request tests assert on canned `mock!` outcomes — they verify the executor's behavior (how it drives the store: `begin` → work → `commit`/`rollback`, and what it does with the results), not the store's. **Strava** is stubbed (canned) for the gear-hydration path.
- **What it delegates to the seam** — the **store I/O** (real Postgres) lives in `tb_sqlx`'s `executor_seam.rs` (see [`sqlx.md`](sqlx.md)); the **domain-level results** live in the `tb_domain` suite (see [`domain.md`](domain.md)); the **Strava JSON → Part mapping** lives in `tb_strava` (see [`strava.md`](strava.md)).

## Coverage

- **Reclaim decision** (`next_action`): prefer a pending write over everything, a ready Strava event when no write, reap with no streams, sleep while streams listen
- **Message selection** (`select_message`): an API write wins over a ready Strava event, a Strava event when no write is pending, idle when the queue is empty and no write, idle when the channel is closed, a Strava error is its own state, limited when a stop is active, a read wins over a ready Strava event, a wake fired before / while parked
- **Housekeeping** (`stop_remaining`): the queue probe reaps queued events past the deadline; an expired deadline reaps nothing
- **Backoff** (`DbBackoff`): doubles until the cap, resets after a success
- **Frame routing** (`push_frame` / `deliver_frame`): non-empty and tombstone-only summaries delivered, empty summaries skipped, user context to the personal stream, shop context to the shop stream / the part owner / not the shop owner (except when the part owner)
- **Per-request** (`run_request`): a read arriving during a sleep is taken immediately, a failed read never arms the backoff and the loop survives, a committed read resets the backoff, `all_stats` round-trips through the loop transaction, gear hydration pushes the created-parts frame, a failed gear arms the backoff like a write, gear hydration without new gear pushes no frame, a shop-context write routes via the pusher, a personal write routes to the own stream
- **Registry / stream lifecycle** (`registry.rs`): `subscribe_shop` rejects a non-owner, the shop stream lifecycle (create / receive / drop), `push_to_shop` is a no-op without a subscriber, `push_to_user` delivers to a running executor / is a no-op without one

## Key infrastructure

All in `executor/executor_tests.rs` (the `#[cfg(test)]` module declared in `executor.rs`).

- **`MockConn`** — a `mockall` `mock!` witness (`#[cfg(test)]`) accumulating the full `Store` seam (nine sub-traits + the method-free `Store` marker) *and* the `StravaStore` surface *and* the `Txn` lifecycle — the `TxnSource::Conn` bound. `#[async_trait]` on each impl block is what makes the `async fn` methods desugar (see [`docs/research/mockall-conn-witness.md`](../../../docs/research/mockall-conn-witness.md) §1); the seam is collision-free (post-recipe: renamed methods, `usages_update(&[Usage])`, `stravauser_update_token` owning its token), so the flat `expect_<name>` API reaches every method on one type. `mockall` is a dev-dependency of `tb_exec` only.
- **`test_context` / `LoopContext`** — the loop's wiring and state, built with a canned `MockConn`, a request channel, and a dummy frame sink; the per-request tests (`run_request`) drive one message through the real loop.
- **`ready_probe`** — the canned queue-probe future the `select_message` / `next_action` units test against.
- **fixtures** — `request(...)` (a `Request` bundling a write/read and its oneshot reply), `part_test_bike` / `user_record` / `shop_owned_by` / `summary_owned_by` / `summary_with_service` / `non_empty_summary` / `shop_source` (canned domain values).

## Writing new tests

```rust
use super::{LoopContext, run_request, select_message, next_action};
use crate::message::Request;

#[tokio::test]
async fn my_test() -> TbResult<()> {
    let mut conn = MockConn::new();
    conn
        .expect_partid_get_part()
        .returning(move |_| Ok(/* a canned Part row */));
    let ctx = test_context(conn);
    // ... drive a `Request` through `run_request` / `select_message` / `next_action`,
    // assert on the frame pushed and the oneshot reply
    Ok(())
}
```

- `#[tokio::test]` for async; return `TbResult<()>` so `?` propagates domain errors with context.
- Script only the store methods the operation under test calls (`MockConn` has no state — an unscripted call panics, which is a test bug, not a store bug).
- The pure control-flow units (`next_action`, `select_message`, `stop_remaining`, `DbBackoff`) take a channel and dummy futures — no `MockConn` at all.
- Canned domain values come from the fixtures (`summary_owned_by`, `non_empty_summary`, …); the suites have no wall clock, so every fixture time is a fixed value.

## Gotchas

- **The store is canned, the logic is real.** The per-request tests (`run_request`) assert on canned `mock!` outcomes (the store returns the values the test sets on `expect_*`): they verify the executor's behavior — how it drives the store (`begin` → work → `commit`/`rollback`) and what it does with the results (frame routing, the backoff discipline, the oneshot reply) — not the store's internal behavior. The store's behavior (and the domain logic that runs over it) is verified against real Postgres in `tb_sqlx`'s seam suites (see [`sqlx.md`](sqlx.md)).
- **`mockall` is a `tb_exec` dev-dependency only** — `MockConn` lives in the `#[cfg(test)]` module, so no production `pub` surface is widened.
- **`#[async_trait]` on each `impl` block** is what makes the `async fn` methods desugar in the generated `mock!` code — a plain `impl` on the async methods does not compile.
- The loop computes no domain state of its own: it drives the existing `ApiWrite` dispatch and `tb_strava` event processing inside transactions it owns, so a test that wants a domain result asserts on the frame the loop pushes, not on a store row.
