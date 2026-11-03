# ADR-0011: The Strava seam takes a session only where it receives one

## Status

Accepted.

## Context

The domain interface is session-based: every domain operation takes the acting session — the caller's identity and context for the operation's duration. `tb_strava` sits on top of the domain and is driven from two kinds of context:

- **Real acting sessions.** The API-write lane (the request's session — e.g. gear hydration's `update_gear`) and the Registry's direct ops (the `strava_url` pair, `user_deauthorize`, `user_delete`, the admin `sync_users`). These carry context that can be behavior-bearing: the write lane's active shop lands on the gear hydration creates (`Part::create` is shop-scoped), and the admin ops gate on `is_admin()`.
- **The event lane.** The executor's `run_event` processes the user's `strava_events` queue. It has no caller and no context: the queue is per-user, the data is the user's own, no shop applies (ADR-0009), and it can't be an admin operation. The only real information is the user id.

Today the seam is uniformly session-based, so the event lane wraps its bare id in `ExecutorSession` (in `tb_exec`) — a `Session` impl whose `shop()` and `is_admin()` are hardwired to `None`/`false`. The wrapper is a fake at the seam: it asks the executor to pretend to be a session the lane doesn't have, and it hides the fact that the event lane's context is entirely lane-determined.

## Decision

**The `tb_strava` seam takes a `Session` only where it receives an acting session; everywhere else it takes a `UserId`.**

- The event-lane entry points and their chain take `UserId`: `event::process` and its private chain (`process_activity`, `process_hook`, `sync`, `process_sync`, `next_activities`), `activity::upsert_activity`, `send_to_tb`, `into_activity`, `delete_activity`, and the HTTP helper `http::request_json` (a per-user token lookup — nothing context-varying flows through it).
- Signatures whose production callers hand a real acting session keep `Session`: `gear::into_partid` (shared with the write lane, whose shop context is behavior-bearing through the shop-scoped `Part::create`), `activity::strava_url`, `gear::strava_url`, `user::update_gear`, `user::user_deauthorize`, `user::user_delete`, `event::sync_users`.
- Where the id-only chain crosses into a session-typed call, it constructs `OnBehalf` — a private, `Copy`, minimal `Session` impl in `tb_strava` (`user_id` = the id, `shop() = None`, `is_admin() = false`). The crossings are `into_activity` → `into_partid`, `send_to_tb` → `Activity::upsert`, and `delete_activity` → `ActivityId::delete`. The type encodes the event lane's actual context — not a guess — and lives at the boundary where that context is known.
- `ExecutorSession` is deleted from `tb_exec`; `run_event` passes `ctx.user_id` to `process`.

The domain interface is unchanged: domain operations still take the acting session. The session is the domain's language; `tb_strava` speaks it only when it talks to the domain.

## Rejected alternatives

- **Uniform session at the seam (status quo).** Forces the id-only lane to construct a fake session at the call site; the parameter type lies about the information the caller has.
- **`UserId` everywhere in `tb_strava`, including the write-lane and direct-op signatures.** Loses behavior-bearing context: the write lane's active shop no longer reaches the shop-scoped `Part::create` / `PartId::part`, and the admin gates (`is_admin()`) have nothing to read.
- **Relaxing the domain to take ids.** The acting session is the domain's concept of "on whose behalf"; the API-write lane has real sessions with real context, and the domain's shop-scoped and admin-gated ops need it.

## Consequences

- `tb_strava`'s surface splits by lane: `process` and `upsert_activity` take `UserId`; the direct-op and write-lane entries take `Session`. A new strava entry point takes a `Session` iff its caller has an acting session.
- `OnBehalf` is crate-private to `tb_strava`; no new public type is introduced.
- `ExecutorSession` and its construction in `run_event` are deleted; the executor no longer names a session type in the event lane.
- No behavior change: every `OnBehalf` crossing encodes the event lane's actual context (personal, non-admin — ADR-0009); the write-lane and direct-op paths are untouched.
