# The message-based executor loop — executable spec

The executable spec for the design locked in ADR-0005 and the five tickets of the "Message-based executor loop for the domain — design" map ([#438](https://github.com/hcrohland/tendabike/issues/438)–[#442](https://github.com/hcrohland/tendabike/issues/442)). This is the record an execution effort implements from; it is not itself the implementation.

## Overview

A **per-user executor** — a long-lived task in a new `tb_exec` crate — replaces the 60-second poll with a continuous loop that drives the domain. The loop consumes two message sources:

- **API writes** — an in-memory channel, one per user. Every mutating API route enqueues an `ApiWrite` and awaits a oneshot.
- **Strava events** — the existing `strava_events` DB queue, read with the latest-per-object dedup.

It runs each message in **one transaction** (`begin` → domain op → `commit`/`rollback`) and **pushes the resulting `Summary` to the client over SSE**. The client opens a session-scoped `GET /api/user/stream`, merges every frame, and the 60-second poll and `GET /strava/hooks` are removed.

**Single node.** One process owns a user's executor and their SSE streams. Durable state lives in Postgres (the `strava_events` queue, the global `Stop`); the in-memory structures (the executor task, the API-write channel) are per-process and respawn on demand. Multi-node is out of scope (ADR-0005).

The domain-flow claim is unchanged and generalized: the domain computes entity state; triggers drive domain operations. The executor is a new *trigger* — a continuous one — that drives the same domain write operations a user would run. It computes nothing of its own.

## 1. The shape of the change

One crate is added. The dependency graph, layered (each layer depends only on the layers below it):

```
tb_axum    → tb_domain, tb_strava, tb_sqlx, tb_exec   (new edge: tb_exec)
tb_sqlx    → tb_domain, tb_strava, tb_exec            (new edge: tb_exec)
tb_exec    → tb_domain, tb_strava                     (new crate)
tb_strava  → tb_domain
tb_domain  → (nothing)
```

- `tb_exec` (new) → `tb_domain` (the `ApiWrite` enum, `Store`, `Summary`) + `tb_strava` (the `Event`, `StravaStore`).
- `tb_sqlx → tb_exec` (new edge; implements the `Txn`/`TxnSource` traits).
- `tb_axum → tb_exec` (new edge; wires the loop, the SSE endpoint, the write handlers).
- `tb_domain` depends on **nothing new** — the `ApiWrite` dispatch is bounded by `Store`, not `Txn`, so there is no `tb_domain → tb_exec` cycle (the `MemStore` must not implement `Txn`).

The `Store` marker stays method-free (ADR per #432). The transaction lifecycle the loop drives is named by the new `Txn`/`TxnSource` traits in `tb_exec`; the `Store` marker's doc gains the executor as a lifecycle driver.

## 2. The `Summary` as the id-keyed map (`tb_domain`)

The `Vec`-based `Summary` struct is dropped; `SumHash` is renamed to `Summary` and becomes the single payload type (glossary term = type name). This is a domain-wide ripple that the whole spec builds on.

- **Shape.** `Summary` is a struct of nine id-keyed maps, each `HashMap<Id, Option<E>>`:

  | field | key | value |
  |---|---|---|
  | `activities` | `ActivityId` (i64) | `Option<Activity>` |
  | `parts` | `PartId` (i32) | `Option<Part>` |
  | `attachments` | `String` (`AttachmentDetail::idx`) | `Option<AttachmentDetail>` |
  | `usages` | `UsageId` (Uuid) | `Option<Usage>` |
  | `services` | `ServiceId` (Uuid) | `Option<Service>` |
  | `plans` | `ServicePlanId` (Uuid) | `Option<ServicePlan>` |
  | `part_notes` | `PartNoteId` (i32) | `Option<PartNote>` |
  | `shops` | `ShopId` (i32) | `Option<Shop>` |
  | `users` | `UserId` (i32) | `Option<UserPublic>` |

  `Some(entity)` = live; `None` = deleted (a **tombstone**).

- **Wire.** A uniform JSON object with stringified id keys — `{"parts": {"12": null, "13": {...}}, "services": {"<uuid>": {...}}}` — via a small custom `Serialize` helper, so every collection shares one shape (a naive derive would give pair-arrays for the integer-keyed collections and objects for the string/UUID-keyed ones). The helper serializes each map with `id.to_string()` keys and `null` for `None` values.

- **Merge.** Plain per-id `insert`, last-wins — exactly today's `SumHash` semantics. No precedence rule between `Some` and `None`: a hash holds one entry per id, and no operation composes a delete and an upsert of the same id, so each id in a composed `Summary` is either upserted or deleted — no race.

- **Domain ripple.**
  - Operations return the map directly (the `From<SumHash> for Summary` and `From<Summary> for SumHash` conversions go away).
  - `Add`/`AddAssign` become map+map (per-id `insert`, `None` overwriting `Some` is a delete).
  - The existing activity-ghost zeroing (the `Vec`-era workaround) is redundant and can go.
  - The full read (`UserId::get_summary`) builds the map with all-`Some` entries (the DB holds no deleted rows).
  - Test assertions adapt from `Vec` indexing to map lookups.

## 3. The `ApiWrite` dispatch (`tb_domain`)

- **`ApiWrite`** — a flat enum, one variant per mutating route, entity-prefixed. Each variant carries the operation's parsed arguments (the same values the current axum handlers extract). The variants, by module:

  - **Part** — `PartCreate`, `PartChange`, `PartDelete`
  - **Attachment** — `AttachmentAttach`, `AttachmentDetach`, `AttachmentDispose`, `AttachmentRecover`
  - **Activity** — `ActivityUpdate`, `ActivityDelete`, `ActivityDescend`, `ActivityDefaultGear`
  - **PartNote** — `PartNoteCreateText`, `PartNoteCreateFile`, `PartNoteUpdateText`, `PartNoteUpdateFile`, `PartNoteRemoveFile`, `PartNoteDelete`
  - **Service** — `ServiceCreate`, `ServiceUpdate`, `ServiceDelete`, `ServiceRedo`
  - **ServicePlan** — `ServicePlanCreate`, `ServicePlanUpdate`, `ServicePlanDelete`
  - **Shop** — `ShopCreate`, `ShopUpdate`, `ShopDelete`, `ShopRegisterPart`, `ShopUnregisterPart`, `ShopSubscriptionCreate`, `ShopSubscriptionApprove`, `ShopSubscriptionReject`, `ShopSubscriptionCancel`
  - **User** — `UserOnboardingSync`, `UserOnboardingPostpone` (onboarding only; no generic user CRUD routes exist)

  (~35 variants; the exact set is the full mutating route table in `backend/axum/src/domain/` plus the two onboarding routes. Reads — `GET /api/user/summary`, `GET /api/user`, lists, the types routes — are **not** `ApiWrite` variants; they bypass the loop.)

- **Context.** The `ApiWrite` does **not** carry the `user_id`: the in-memory channel is per-user (one per per-user loop), and the loop already runs in that user's context. The executor supplies the session.

- **`exec`** — `pub async fn exec(write: ApiWrite, session: &mut impl Session, store: &mut impl Store) -> TbResult<Summary>`. It matches on the variant and calls the corresponding domain operation — the same call the current axum handler makes, minus the HTTP extraction. It is bounded by `Store` (not `Txn`), so it stays verified on **both** adapters per the existing seam contract (Postgres the source of truth).

  - The admin routes are **not** `ApiWrite` variants (they are admin-triggered, not user writes): the admin `GET /strava/sync/{id}` routes through the executor as a Strava sync event (§6.4), and the admin `Activity::rescan_all` (`GET /api/activ/rescan`) is all-users maintenance work that stays a direct domain op — a corner case, not a per-user write.

## 4. The `tb_exec` crate

### 4.1 The `Txn` + `TxnSource` traits

The transaction lifecycle is split across two types today (`DbPool::begin` vs `SqlxConn::commit`/`rollback`), and one trait cannot carry methods on two different types, so the seam is **two traits linked by an associated type**:

```rust
#[async_trait]
pub trait TxnSource: Send + Sync {
    type Conn: Store + Txn;
    async fn begin(&self) -> TbResult<Self::Conn>;
}

#[async_trait]
pub trait Txn: Store {
    async fn commit(self) -> TbResult<()>;   // consuming, matching SqlxConn
    async fn rollback(self) -> TbResult<()>; // consuming, matching SqlxConn
}
```

- `#[async_trait]` per the repo convention; the source carries `Send + Sync` (the loop is a long-lived `Send` task holding `&T` across awaits; `DbPool` satisfies it).
- The loop is generic over the source: `run<T: TxnSource>(…) where T::Conn: StravaStore` — the `StravaStore` bound sits on the **loop**, not the trait (the queue read and `process` run on the same conn as the domain op; the trait stays lifecycle-only).
- `MemStore` implements **neither** (the `tb_domain → tb_exec` cycle; its deliberate ergonomic differences — sync `begin`, non-consuming `rollback` — are irrelevant, since nothing in `tb_domain`/`tb_strava` names `Txn`).

### 4.2 The `Message` enum

A thin wrapper unifying the two sources:

```rust
pub enum Message {
    ApiWrite(ApiWrite), // from tb_domain
    Strava(Event),      // from tb_strava
}
```

The loop's `select!` is **biased**: API writes (the in-memory channel) are polled first, then Strava events (the DB queue). Within the in-memory channel, API writes process FIFO; within the DB queue, Strava events process in `event_time` order. The bias guarantees an API write is never queued behind a Strava backoff sleep.

### 4.3 The per-user loop

`run` is the per-user task. One instance per user. Its body:

1. **Spawn context.** The loop holds: the user's `StravaSession` (see §4.4), the per-user API-write channel receiver, the registry of the user's active SSE streams (§4.5), and the backoff state (§4.6).
2. **Select a message.** A biased `select!`:
   - An API write arrives on the channel → process it.
   - Otherwise read the next Strava event from the DB queue (the loop's "get next Strava event" step — the latest-per-object dedup from today's `get_event`, moved here; it is a Strava-side concern, API writes don't dedup).
   - Otherwise (both idle) → sleep; the loop is reaped on idle (§4.4).
3. **Run it in one transaction.** `let mut conn = source.begin().await?;` → dispatch (`exec` for `ApiWrite`; `process` for `Event`, the existing `tb_strava::event::process`) → `commit`/`rollback` on success/failure.
4. **Push the result.** If the `Summary` is non-empty, push a frame to the user's SSE streams (§4.5). Empty frames are skipped.

The loop logs `Processing {event}` / the `ApiWrite` variant before dispatch, so a poison message is named in the log.

### 4.4 The lifecycle (spawn on demand, reap on idle)

- **One per-user task**, spawned on demand: the first SSE connect **or** the first API write for that user spawns it (whichever comes first). A registry in `tb_axum` (a `Mutex<HashMap<UserId, …>>` or equivalent) tracks the live executor per user so a second request finds the running task rather than spawning a duplicate.
- **Reaped on idle.** When the loop has no pending messages and no active SSE streams (or after an idle timeout), it exits cleanly (`Ok(())`). The in-memory channel and the task are dropped.
- **Respawn.** The next SSE connect or API write respawns it. On respawn the loop re-registers the user's active SSE streams (§4.5) and resumes the DB queue on demand (no startup scan — see §4.6).
- **The `StravaSession` source.** The per-user task needs a `StravaSession` at spawn (for `process` and the queue read). It is built from the user's stored `StravaUser` (the refresh token) the same way `RequestSession::create_from_id` does, and refreshed on demand by the existing token-refresh path.

### 4.5 The SSE push

- **Frame shape.** Each SSE frame is the `Summary` (the map JSON, §2) of the completed message — no envelope, no type discriminator, no sequence number. The executor pushes a frame for **every** completed message (Strava events and API writes alike): one rule, and multi-tab sync falls out (a second tab of the same user sees the first tab's writes live).
- **Stream ownership.** The SSE stream is **web-layer-owned** (`tb_axum`) and **outlives** the per-user executor task. There is one stream task per SSE connection (per session/tab). The executor holds a registry of the user's active stream senders; on respawn it re-registers them.
- **Idle keep-alive.** While the executor is reaped (idle), the stream task keeps the connection alive with heartbeat frames — SSE comments (`: ping`), invisible to `onmessage`.
- **Panic teardown.** The stream task selects on the executor's `JoinHandle`: `Err(Panic)` → close the user's streams (so the client reconnects); `Ok(())` (idle reap) → keep heartbeating. See §4.6.
- **A push failure is non-fatal.** If a client disconnects mid-frame, the frame is lost but the message is committed; the oneshot resolves success (the commit succeeded); the client's reconnect + refresh catches the change.

### 4.6 Failure semantics

**API writes (in-memory channel): fail-fast, no retry, no dead-letter.**
The handler enqueues + awaits the oneshot (§6.2); on failure the oneshot resolves with the error → 4xx/5xx via the existing mapping (`NotFound`→404, `Conflict`/`Ambiguous`→409, `DatabaseFailure`/`AnyFailure`→500). The message is consumed — gone; the client is the retry layer (`handleError` → banner; the user retries manually). The oneshot wait is **unbounded**: per-user serialization is settled, the only delay is an in-progress message (bounded by its length), and the biased select guarantees a write is never queued behind a backoff sleep.

**Strava events (DB queue): drop on permanent failure — no dead-letter table.**
- **Activity**: non-transient error (404, parse error) → delete from the queue, log. A 404 means the activity is gone — nothing to import.
- **Sync**: a permanently bad activity in a batch → **skip it, advance the cursor, log**. This fixes today's wedge: the batch loop exits before the cursor advances past the bad activity, so one bad activity wedges the queue in an infinite retry loop. Liveness > completeness; the admin sync endpoint (takes a `time` param) re-covers a gap if the user cares.
- **`TryAgain`** (Strava 429/502/503/504) → the backoff below; the failed event stays queued and is retried after it.
- **`NotAuth`** (401 after a token refresh) → **disable the user's Strava integration** (reuse the existing `StravaId::disable` path from the Athlete deauth) + **drop all their queued events** (`strava_events_delete_for_user`). The data freezes (stale but consistent); the loop idles; no event-by-event 401s.

**The rate-limit backoff — global, in the DB.**
The Strava rate limit is app-wide, and the existing `Stop` mechanism is already global: `insert_stop` creates the `Stop` with `owner_id = 0`, and `strava_event_get_next_for_user` reads `owner_id = ANY([0, user_id])` — one user's 429 already stops every user's drain today. The loop keeps it:
- `TryAgain` → insert the global `Stop` (`now + 900s`) + keep the failed event. **15 min fixed** (the existing tuned value). If a `Stop` already exists, extend its deadline rather than stacking rows.
- Each user's loop reads the global `Stop` via the existing queue read; unexpired → **sleep until expiry as a branch of the biased `select!`** — interruptible by API writes, so the backoff delays only Strava events, never API writes. Expired → delete → continue.
- The DB `Stop` (vs an in-memory deadline) survives a server restart (no 429 burst after a deploy).

**DB failures** (commit fails, pool exhausted — not a rate limit): per-executor in-memory exponential backoff, capped (e.g. 30s); no `Stop` row. The event stays queued (the txn rolled back) and is retried after the backoff.

**A sync enqueued while the backoff is active** (the admin sync, §6.4) **waits** for the backoff to expire — the oneshot wait is unbounded, so the request can hang up to 15 min. Accepted: an admin endpoint, a rare corner, honest semantics (the shared budget is genuinely unavailable). No 429 fast-fail.

**Executor panic — the client is the recovery path.**
**No supervisor.** The stream task selects on the executor's `JoinHandle`: `Err(Panic)` → close the user's streams; `Ok(())` (idle reap) → keep heartbeating. The client's native reconnect (~3s) + full refresh (§7) then respawns the executor, which re-registers the streams. Consequences: the in-flight API write's oneshot is dropped → the handler gets `Canceled` → 500 → the user retries; the in-flight Strava event's txn rolls back → stays queued → retried after respawn.

**No panic-loop guard.** A deterministic poison event (panics every time) produces a perpetual reconnect/refresh cycle: the data stays correct (every refresh succeeds), and the symptom is a persistent spinner — a visible alarm, not a silent one. The user is the guard: for a personal tracker, a visible stuck state the owner investigates beats silent self-healing that drops data. Recovery: the loop logs `Processing {event}` before dispatch, so the log names the poison; the owner deletes it from `strava_events` (a restart alone does not clear it — the queue is in the DB) or fixes the bug.

**Server restart.**
All in-memory state dies (executor tasks, API-write channels, SSE streams); the DB queue and the global `Stop` survive.
- In-flight API writes are lost → the client's fetch fails (network error) → banner → **the user retries manually** (no auto-retry; the mutation handler is `myfetch(...).catch(handleError)`).
- A write that committed but whose 204/frame never arrived is caught by the client's reconnect-then-full-refresh (§7 ordering) — the single recovery mechanism, self-healing.
- The Strava queue resumes **on demand** (first SSE connect or first write spawns the executor) — no startup scan: a scan would make inactive users' backlogs compete with active users for the shared Strava budget (scan drains 429 → global 15-min pause → active users wait too), for a benefit the user only sees when they open the app anyway. Today's behavior is identical (events waited for the client's poll).

**Webhook ingest is unchanged.** `create_event` (`POST /strava/callback`) still only stores in the DB queue + fires an in-memory wake signal; a failed ingest 500s to Strava (Strava's own retry policy applies). The wake signal is non-critical — the DB queue is the source of truth; the loop picks the event up on its next wake.

## 5. The `tb_sqlx` `Txn` impl

- **`DbPool: TxnSource`** (`Conn = SqlxConn<'static>`) and **`SqlxConn<'static>: Txn`** — both in `tb_sqlx` (the new `tb_sqlx → tb_exec` dependency). The trait impls **delegate to the existing inherent methods** (`DbPool::begin`, `SqlxConn::commit`/`rollback`); the inherent methods stay — the web handlers (reads) and the seam suite keep calling them on the concrete types; the traits exist for the generic loop.
- **`SqlxConn::rollback`'s doc** ("nothing in the workspace calls it yet") goes stale — the loop calls it on a failed message. Update it.
- **Test placement.** The executor's Postgres suite lives in `tb_sqlx` — a new file (`tests/executor_seam.rs`) beside `store_seam.rs`. `tb_sqlx` already depends on `tb_exec`, so it can name both `DbPool` and the executor; it reuses the scratch-DB infra (factoring the setup into a shared helper both suites use is a spec detail); `#[ignore]` + fail-loud + the CI `--include-ignored` run pick it up with no workflow change. Rejected: a `tb_exec` dev-dependency on `tb_sqlx` (a legal dev-dep cycle, but no precedent in the workspace) and `tb_axum` (its tests are all in-memory by convention).

## 6. The `tb_axum` wiring

### 6.1 The SSE endpoint

`GET /api/user/stream` — session-scoped, next to `/api/user/summary`, same session extraction (`RequestSession`). `EventSource` sends the session cookie same-origin; no headers needed. The handler:
1. Extracts the session (→ the `UserId`).
2. Spawns/gets the user's executor (§4.4) and registers this connection's stream sender in the user's stream registry.
3. Streams SSE: forwards frames from the executor, heartbeats when idle, and tears down on client disconnect (deregistering the stream; reaping the executor if it was the last stream and there's no work).

### 6.2 The write handlers

Every mutating handler changes from "begin → op → commit → return the `Summary`/entity" to **enqueue + await (oneshot)**:
1. Extract the session + JSON/path args (unchanged).
2. Build the `ApiWrite` variant from the extracted args.
3. Enqueue it on the user's executor channel with a oneshot; await the oneshot (unbounded).
4. Return a **status**, not a body:
   - **Creates → `201` + the created entity** (unchanged). `Part::create()` etc. keep returning the entity — the wizard chains `create().then(p => attachPart(p, hook))` with the server-assigned id. The entity also arrives via the stream (idempotent double-merge).
   - **All other mutations → `204 No Content`** — updates, deletes, attach/detach/dispose/recover, register/unregister, service redo. The stream frame is the sole delivery of the change; the client's mutation handlers collapse to `myfetch(...).catch(handleError)`. (`checkStatus` already maps 204 → `null`.)
   - **Onboarding sync/postpone → `200` + `User`** (unchanged — a `User` is not in any `Summary`, so the stream can't carry it).
   - **Reads** — unchanged, bypass the executor.

This revises the map's settled line *"API response — enqueue + await (oneshot); the handler's external contract is unchanged"* to: **enqueue + await (oneshot); the handler returns a status; the change rides the stream.**

### 6.3 The removals

- **`GET /strava/hooks`** — removed outright. Its only client consumer is the 60s poll, which goes away; keeping it as a manual drain would be a second write path bypassing the executor's per-user serialization.
- **The 60s poll** — removed from the client (§7).

### 6.4 The admin sync through the executor

The admin `GET /strava/sync/{id}` must route through the executor (enqueue a sync event + await → `204`) — a direct `process()` call would bypass the per-user serialization. It awaits the oneshot (unbounded) and returns `204`.

The admin `Activity::rescan_all` (`GET /api/activ/rescan`) is the one mutating route that does **not** route through the executor: it is admin-triggered and all-users, so it does not fit the per-user model and does not conflict with any single user's serialization. It stays a direct domain op (`begin` → `rescan_all` → `commit`).

## 7. The client SSE listener

A new module (e.g. `frontend/src/lib/stream.ts`) owns the stream, replacing the poll in `Header.svelte`.

- **Frame shape / merge.** Each frame is the `Summary` (map JSON, §2). `updateSummary` revises from "upsert by id, never removes rows" to: per entry — `null` → `collection.deleteItem(id)`, else upsert. The statemap's delete function is the existing `deleteItem`; the `null` is a uniform predicate, so no per-type analysis is needed. `setSummary` (hydration) unwraps the map values; the DB holds no deleted rows, so hydration carries all-`Some` entries.
- **Reconnection.** On a dropped stream: **re-establish the stream first, then issue the full `GET /api/user/summary` refresh** (`setSummary`). The ordering closes the gap: anything committed before the reconnect is caught by the refresh (its read is after the reconnect); anything committed after is delivered on the live stream. No poll fallback.
  - Same rule on initial page load: open the stream, hydrate on `open` (replacing today's hydrate-then-poll order in `initData`).
  - The catch-up refresh mirrors the current view's scoping — the same `refresh(getShop()?.id)` call the manual refresh uses — and is retried on failure (upserts keep flowing on the live stream meanwhile; deletions wait for it).
- **Backoff.** Native `EventSource` auto-reconnect (~3s). After **5 consecutive failed opens**, the client fires a real `fetch` probe (`GET /api/user` through `myfetch`) so the existing 401 → login-redirect handling can fire — `EventSource` never exposes the response status, so this is the only way to distinguish a dead session from a network blip. Other probe failures → keep native retrying.
- **Stream lifetime.** The stream is web-layer-owned and outlives the per-user executor task (reaped on idle): heartbeat frames keep it alive through idle, and the executor re-registers the stream on respawn. Reconnect + refresh happens only on a real drop.
- **UI.** The existing avatar spinner (`hook_promise`) is reused — shown while the stream is connecting/reconnecting and while the catch-up refresh is in flight; the global error banner appears only when the auth probe fails or failures persist.
- **The drain loop is removed.** The `do/while (data["activities"].length > 0)` drain loop in `Header.svelte` is removed — the client never drains; it merges every frame. This resolves the two `domain-flow.md` review items: #305 (drain only continues while activities arrive) and #307 (drain reads the raw `data["activities"]` array).
- **The manual `fullrefresh()`** drops its `.then(poll)`; the stream runs independently.

## 8. Removals (consolidated)

- The 60-second poll (`Header.svelte` `poll()` + `hook_timer`).
- `GET /strava/hooks` (the `webhook::hooks` handler + its route).
- The `Vec`-based `Summary` struct (replaced by the map, §2).
- The `do/while` drain loop in `Header.svelte`.
- The activity-ghost zeroing (redundant once `Summary` is a map, §2).

## 9. Cutover

**Hard cutover in one release** — SSE in, poll out, `/strava/hooks` removed. Frontend and backend ship in one image; tabs left open across the deploy run the old frontend: their poll 404s → error banner, the poll loop stops (a failed fetch never reschedules), writes still work (handler semantics unchanged), the tab is blind to Strava updates until reload — self-healing. No feature flag, no two-phase.

## 10. Verification

- **The `ApiWrite` dispatch: both adapters.** It is `Store`-bounded in `tb_domain`, so it stays verified on `MemStore` + `SqlxConn` per the existing seam contract (Postgres the source of truth). Add the dispatch to the in-memory suite (`tb_domain`) and to the seam suite's representative set.
- **The loop: Postgres-only + a pure brain.** The loop's seam integration (begin → op → commit/rollback on a real transaction) is verified only against `SqlxConn` in the new `tb_sqlx` Postgres suite (`tests/executor_seam.rs`). To keep fast in-memory coverage without a fake, the loop's control flow (message selection, backoff state machine, frame construction) is structured as **pure units, unit-tested in `tb_exec` with no store** (no `#[ignore]`, no database). No in-memory store fake: `MemStore` cannot implement `Txn` (the cycle), and a composite fake would require `tb_strava` to expose its `#[cfg(test)]`-only doubles behind a new `test-support` feature — a public surface we don't add for this.
- **The client.** The SSE listener, the tombstone merge, and the reconnect-then-refresh ordering are covered by the existing vitest suite (the `mapable`/`user` tests adapt to the map shape).

## Out of scope

- **Execution** — building the executor is a separate effort; this spec is the design record only.
- **The `domain-flow.md` rewrite** — the pattern bible's lane 3 (the 60s poll + drain) and the `hook` overload note (the word is overloaded with the Strava event-drain endpoint) need a rewrite to the executor + SSE once the executor is built. That is a consequence of the design that lands with the execution effort, not part of this design record. The spec's §7 already resolves the two `domain-flow.md` review items (#305, #307) that the rewrite will absorb.
- **The Strava oauth flow, the HTTP session mechanism, the full route table** (per `domain-flow.md`).
- **Multi-node distribution** — ruled out of this effort (ADR-0005).
- **Concurrency as a goal** — per-user serialization is a side effect of the single per-user loop, not a design target.
