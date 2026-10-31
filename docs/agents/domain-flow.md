# Domain Flow

How the Tendabike client stays in sync with the backend. Read this when adding or modifying a mutating operation — a domain operation, an endpoint that serves it, or its client handler. Terminology: `GLOSSARY.md`.

## The claim

The domain layer (`backend/domain`) is the only thing that computes entity state. Every mutation — user-triggered or Strava-triggered — computes its side effects inline, in one transaction, and produces the `Summary` of everything it touched. Mutations never run in the request handler: **the API write handlers and the Strava webhook are only _triggers_ that enqueue messages to the user's executor**, a long-lived per-user task (`backend/exec`, ADR-0005). The executor drives the same domain write operations a user would run, one message per transaction, serialized per user, and computes nothing of its own. The executor publishes each completed message's `Summary` as a frame on the user's SSE stream; **the client's single path into entity state is `updateSummary` over those frames** (plus the one-shot full summary of hydration/reconnect). The client may combine entity state to compute derivatives (groupings, counts, per-view projections); entity state itself always comes from the domain. Plan due-ness is the representative derivative: the client computes it from the merged entity state (plans, services, usages, attachments) against the clock, and it changes with the passage of time, not only on mutation — so it is a client derivative, never a stored server value (ADR-0002).

## The lanes

1. **Hydration** — `GET /api/user/summary[?shop=]` returns the user's full state; the client replaces every map (`setSummary` in `frontend/src/lib/user.ts`). The handler also runs `StravaUser::update_gear` in the same transaction, so a read can change state (see review 01 below). It is the client's catch-up snapshot: on first load it runs in `initData`, in parallel with the stream's connect (not after it); after every stream reconnect it runs once the stream is open again. Frames that race the first snapshot are buffered in the stream until it lands (`markHydrated`).
2. **Writes** — POST/PUT/DELETE on the `/api` routes. The axum handlers are thin: extract session and JSON, enqueue the `ApiWrite` on the user's executor (`Registry::write`, `backend/axum/src/stream.rs`), and answer with the executor's outcome — creates answer `201` with the created entity (its id is server-assigned and the caller needs it), the Garmin descend answers `200` with the match report `{good, bad}` (the rows the domain matched and skipped — an operation result, not `Summary` state; the matched activities' state still rides the stream frame), every other mutation answers `204`: the change reaches the client — every open tab, this one included — as the stream frame the executor publishes.
3. **Strava** — `POST /strava/callback` receives Strava events and _only queues them_ in the `strava_events` database table, then wakes the user's executor — the wake interrupts a running executor's idle wait so the event is processed at once; it never spawns (a reaped executor drains the queue on its next spawn — the DB queue is the source of truth). The executor drains the queue in order: `process()` in `backend/strava/src/event.rs` executes the corresponding domain write operations (import activity, run a sync fetch), and the executor pushes each resulting `Summary` as a frame. The browser's lane in is `GET /api/user/stream` (session-scoped SSE, next to `/api/user/summary`): each `data:` frame is the full `Summary` JSON of one completed executor message — no envelope, no sequence. **The executor's lifecycle:** the executor is reaped when no stream is attached (a write-only spawn reaps as soon as its work is done), and an attached stream keeps it alive — the 60s idle window is the cadence at which the idle loop re-probes the `strava_events` queue, not a reap trigger (spec §4.4, amended — issue #446). The stream stays alive on `: ping` heartbeat comments, invisible to `onmessage`, while the executor is idle — and across the one reconnect frame of the connect-time race (if the executor reaps before a stream's receiver attaches, the stream gets one `: ping` and re-subscribes; the respawn settles it). The old poll (`GET /strava/hooks`, 60s) is gone; the webhook never talks to the browser directly.
4. **The client stream** — `frontend/src/lib/stream.ts` opens the `EventSource` on `/api/user/stream` and feeds every frame to `updateSummary`. The stream opens only for an authenticated user: the `Header` starts it once `getUser()` is known — an anonymous session gets the About page (the 401 redirect), never a stream. On first load the catch-up snapshot (`GET /api/user/summary` via `refresh` → `setSummary` in `initData`) runs in parallel with the connect, and the stream buffers the frames that race it until the snapshot lands; on every reconnect the client goes stream-first, then the snapshot — the snapshot reconciles anything the stream missed. The native `EventSource` reconnects by itself (~3s); because it never exposes response status, after five consecutive failed opens the stream probes liveness with a real `GET /api/user` fetch — the probe is where the 401 → login redirect can fire. The `Header` keeps the avatar spinner in step with the initial snapshot and every reconnect snapshot (`hook_promise`), and the stream stops on unmount.

## Notifications

A `Notification` is a message addressed to a single recipient, created as a side effect of a domain operation — the subscription lifecycle events (`sub_request`, `sub_response`, `sub_cancelled`) and part registration/unregistration (`part_registered`, `part_unregistered`) — and never directly by the user (issue #586). It is stored per recipient and rides the `Summary` as its own map (the tenth, `notifications`): the operation's transaction creates the row and adds it to the returned `Summary`, like any other side effect.

**Cross-recipient delivery.** After a write commits, the executor inspects the `Summary`'s notification map and routes each entry to its recipient's personal stream via `push_to_user` (the same routing the shop-context frames use — spec #517, ticket #584). The acting user's own notifications ride the normal frame. A recipient whose executor is not running (offline) gets nothing live: the row is in the DB, and `GET /api/user/summary` carries all of the user's notifications, so it reaches them on the next hydration (lane 1). There is no retention cap.

**Lifecycles and reads.** A notification's lifecycle decides what a read does. **Persistent** (the subscription kinds) — a read marks it `seen` and it stays in the history; **transient** (the part kinds) — a read deletes the row, and the frame carries its tombstone. **Ephemeral** notifications (client-side errors) exist only on the client and never cross the wire. `read` (per id) and `read-all` are executor-routed mutations — `204` plus the frame, like any other write. The client's unread count is the unread persistent plus all transient notifications.

## The domain layer

- **`register` is the primitive.** `Activity::register(Factor::Add | Sub)` (`backend/domain/src/entities/activity.rs`) calls `Attachment::register_activity`, which walks the gear's attachments active at the activity's time, updates the `Usage` rows and the parts' `last_used` timestamps, and returns the `Summary` of the affected parts and usages.
- **Side effects compose.** `Summary` implements `Add`/`AddAssign` (per-id, last-wins; `backend/domain/src/summary.rs`). Replacing an activity is `register(Sub) + register(Add)`.
- **One transaction.** Each operation takes a single store; the mutation, its side effects, and the returned `Summary` are one unit.
- **The side-effecting operations** live on the entities: `attach_assembly`, `detach_assembly`, `dispose_assembly`, `recover_assembly` (`entities/attachment.rs`), `Activity::upsert/update/delete`, `Shop::register_part`, and the like. Every one of them returns the `Summary` of everything it touched.
- **The bare-entity operations** touch exactly one entity and return it. The partnote operations (`entities/partnote.rs`) are the reference: `PartId::notes/note_create_text/note_create_file` and `PartNoteId::note/update_text/update_file/remove_file/delete` each return the bare `PartNote`, and `PartNote::file` fetches the file bytes on demand — file bytes are never part of any `Summary`.

## The store seam

The domain layer is storage-agnostic: every operation goes through the store traits in `backend/domain/src/traits/`, and the Postgres store (`SqlxConn`, `backend/sqlx/src/store/`) is the production adapter. The DB-free test suites run the domain logic over `mockall` `mock!` doubles (the `MockStore` in `backend/domain/src/test_support/`), not a second store adapter.

The `Store` trait is a marker naming a complete store (all nine sub-traits); it carries no methods. The transaction lifecycle — `begin`, `commit`, `rollback` — is inherent on the concrete store (`SqlxConn`) and is driven only by the web layer, the executor loop, and the tests — never by the domain.

The contract: a domain operation must pass on the `mock!` double (the DB-free suites) and on the Postgres store (the sqlx seam), and where they disagree the **Postgres behavior is the source of truth** — the database is what production runs. Where a rule was unified, the trait docs name it (the attachment path rules in #407, the activity path rules in #408, and the 30-minute offset rounding in #409).

The contract is enforced in CI: the required `rust` job runs both suites (the seam suite against a real Postgres service); a red seam blocks the PR. See [`backend/docs/tests/sqlx.md`](../../backend/docs/tests/sqlx.md) for the seam suite's mechanics.

## The write contract

- An operation that **creates** touches exactly one entity and answers `201` with the created entity — the id is server-assigned, so the caller needs the response.
- The **Garmin descend** (`POST /api/activ/descend`) answers `200` with the match report `{good, bad}` — the CSV rows the domain matched and skipped. The report is an operation result, not `Summary` state: the matched activities' state reaches the client as the executor's `Summary` frame like any other write. The executor's oneshot reply carries both (a `WriteOutcome`: the `Summary` plus, for the descend, the `DescendReport`); the handler answers the report, the frame carries the state.
- Every operation with **side effects** answers `204`; the change reaches the client as the executor's `Summary` frame over the stream.
- The **admin rescan** (`GET /api/activ/rescan`) is the one mutating route that does not route through the executor — all-users maintenance, the spec #446 §3/§6.4 carve-out. It stays a direct domain op (`begin` → `rescan_all` → `commit`) and answers `204`. No frame carries its effect, so after the commit it stops all live executors (frame and event channels closed, task aborted): every open stream ends, and every client re-hydrates via the native reconnect + catch-up snapshot (lane 1) — the admin client does no manual refresh.
- **Minimality:** operations touch only the necessary pieces and never deliberately regenerate whole state. They stay lightweight; the client-side merge (`updateMap`) exists precisely so frames can be partial. When in doubt, the domain returns a `Summary` — but only of what was touched.

## The client handler (pattern)

`frontend/src/lib/part.ts` is the reference implementation: an entity class, a `mapableState` collection, and fetch calls that only await the write — `myfetch(...).catch(handleError)`. The response is not merged: the change rides the stream frame, and `updateSummary` (in `frontend/src/lib/user.ts`) is consumed by exactly one place in the app, `frontend/src/lib/stream.ts` — the frame handler. The one exception is `Part.create`, which keeps the `201` body because the wizard chains on the server-assigned id. `mapableState` (`frontend/src/lib/mapable.svelte.ts`) is the collection factory: a `$state` record of entities by id with the write operations attached — `setMap` replaces the whole record, `updateMap` merges, `deleteItem` removes. Because the operations are keys of the record, a state collection must not be enumerated with `Object.values`/`filterValues`; use `stateValues`, which skips the operation keys. The full state-object mechanics live in [state-object.md](state-object.md).

## Recipe: add a new mutating operation

1. **Domain** — write the operation as a method on the entity in `backend/domain/src/entities/`. Compute side effects inline in one transaction (through `register`/`Factor` when usage is involved). Return the `Summary` of everything touched, or the bare entity if nothing else is affected. Done when the response covers every entity the operation touched — and nothing it didn't.
2. **Presentation** — a thin axum handler in `backend/axum/src/domain/<module>.rs`: extract session and JSON, enqueue the `ApiWrite` on the user's executor (`Registry::write`), and answer `201` + entity for creates, `204` otherwise. It computes nothing and does not await a domain call directly.
3. **Client** — a fetch call in `frontend/src/lib/<name>.ts`: `myfetch(...).catch(handleError)` (creates return the `201` entity). The handler merges nothing — the change arrives over the stream. Done when the frame for the operation is what updates the maps.
4. **Both stores** — verify the operation on both adapters (the store-seam contract above): run the in-memory suite (`tb_domain`) and the seam suite (`store_seam`, with a `SCRATCH_DATABASE_URL`) against it, and where the operation is a new path, add it to the seam suite's representative set. Done when both suites pass on the one rule — and where they disagree, fix the in-memory store to the Postgres behavior, the source of truth.

To add the _entity_ itself, follow the implementation steps in [`new-entity.md`](new-entity.md).

## To be reviewed

These asymmetries are filed for review; do not assume they are intended design:

1. `GET /api/user/summary` piggybacks `update_gear` in the same transaction — every hydration can mutate state. → [issue #306](https://github.com/hcrohland/tendabike/issues/306)

## Out of scope here

- The Strava oauth flow: `backend/axum/src/strava/oauth.rs`
- The HTTP session mechanism: `backend/axum/src/strava/session.rs`
- The full route table: `backend/axum/src/domain.rs` plus each module's `router()`
