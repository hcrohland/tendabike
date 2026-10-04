# Domain Flow

How the Tendabike client stays in sync with the backend. Read this when adding or modifying a mutating operation — a domain operation, an endpoint that serves it, or its client merge. Terminology: `CONTEXT.md`.

## The claim

The domain layer (`backend/domain`) is the only thing that computes entity state. Every mutation — user-triggered or Strava-triggered — computes its side effects inline, in one transaction, inside a user session, and returns the `Summary` of everything it touched. API routes and the Strava drain are only _triggers_ that drive domain operations; the drain computes nothing of its own — it drives the same domain write operations a user would run. The client merges responses into entity state and may combine entity state to compute derivatives (groupings, counts, per-view projections); entity state itself always comes from the domain. Plan due-ness is the representative derivative: the client computes it from the merged entity state (plans, services, usages, attachments) against the clock, and it changes with the passage of time, not only on mutation — so it is a client derivative, never a stored server value (ADR-0002).

## The lanes, as triggers

1. **Hydration** — `GET /api/user/summary[?shop=]` returns the user's full state; the client replaces every map (`setSummary` in `frontend/src/lib/user.ts`). The handler also runs `StravaUser::update_gear` in the same transaction, so a read can change state (see review 02 below).
2. **Writes** — POST/PUT/DELETE on the `/api` routes. The axum handlers are thin: extract session and JSON, call the domain operation, return what it returned — an entity or a `Summary`.
3. **Strava** — `POST /strava/callback` receives Strava events and _only queues them_ in the database. `GET /strava/hooks` (session-scoped) drains the user's next queued event: `process()` in `backend/strava/src/event.rs` executes the corresponding domain write operations (import activity, run a sync fetch) and returns the `Summary` of what changed, or an empty `Summary` when idle. The client polls it every 60s in `frontend/src/Header.svelte` and drains in a loop while activities keep arriving. There is no push channel to the browser; the webhook never talks to it directly.

## The domain layer

- **`register` is the primitive.** `Activity::register(Factor::Add | Sub)` (`backend/domain/src/entities/activity.rs`) calls `Attachment::register_activity`, which walks the gear's attachments active at the activity's time, updates the `Usage` rows and the parts' `last_used` timestamps, and returns the `Summary` of the affected parts and usages.
- **Side effects compose.** `Summary` implements `Add` (via `SumHash`, `backend/domain/src/entities/summary.rs`). Replacing an activity is `register(Sub) + register(Add)`.
- **One transaction.** Each operation takes a single store; the mutation, its side effects, and the returned `Summary` are one unit.
- **The side-effecting operations** live on the entities: `attach_assembly`, `detach_assembly`, `dispose_assembly`, `recover_assembly` (`entities/attachment.rs`), `Activity::upsert/update/delete`, `Shop::register_part`, and the like. Every one of them returns the `Summary` of everything it touched.
- **The bare-entity operations** touch exactly one entity and return it. The partnote operations (`entities/partnote.rs`) are the reference: `PartId::notes/note_create_text/note_create_file` and `PartNoteId::note/update_text/update_file/remove_file/delete` each return the bare `PartNote`, and `PartNote::file` fetches the file bytes on demand — file bytes are never part of any `Summary`.

## The store seam

The domain layer is storage-agnostic: every operation goes through the store traits in `backend/domain/src/traits/`, and two adapters implement them — the in-memory store (`MemStore`, `backend/domain/src/test_support/`) and the Postgres store (`SqlxConn`, `backend/sqlx/src/store/`).

The `Store` trait is a marker naming a complete store (all nine sub-traits); it carries no methods. The transaction lifecycle — `begin`, `commit`, `rollback` — is inherent on the concrete adapters and is driven only by the web layer and the tests, never by the domain.

The contract: a domain operation must pass on **both** adapters, and where they disagree the **Postgres behavior is the source of truth** — the database is what production runs, so the in-memory store follows it, never the other way around. Where a rule was unified, the trait docs name it (the attachment path rules in #407, the activity path rules in #408, the in-memory store's transactionality and 30-minute offset rounding in #409).

The contract is enforced in CI: the required `rust` job in `.github/workflows/test.yml` is the gate. It runs the in-memory suite against `MemStore` and the seam integration suite (`backend/sqlx/tests/store_seam.rs`) against a real Postgres service — the standard prepopulated fixture, a representative set of domain operations through `SqlxConn`, and the same domain-level assertions the in-memory suite makes — so a red seam blocks the PR in the one required gate.

The seam suite manages its own scratch database at `SCRATCH_DATABASE_URL`: on first use it force-drops any database left behind by a previous run, creates a fresh one, runs the migrations, and seeds the fixture; nothing is dropped at the end of a run, so every run — including a rerun after a failure — starts from the same clean slate. The suite is ignored by default and runs with `cargo test -- --include-ignored` (the `rust` job does exactly that against its Postgres service), and whenever it cannot run — no `SCRATCH_DATABASE_URL` on the machine, or a scratch database that cannot be prepared — it fails loudly instead of skipping, so the gate cannot pass with nothing verified. The suite never connects to `DATABASE_URL` or `DB_URL`, which point at the developer's working database — the original `DATABASE_URL`-based design seeded that database in place, and a local run destroyed real data. It reads the two variables only to refuse a collision: a scratch URL that equals either fails the run instead of dropping real data. To run the suite locally, set `SCRATCH_DATABASE_URL` to a disposable database whose user holds createdb rights.

## The write contract

- An operation that returns a **bare entity** touches only that entity.
- An operation with **side effects** returns the `Summary` of everything it touched — or the client silently drifts.
- **Minimality:** operations touch only the necessary pieces and never deliberately regenerate whole state. They stay lightweight; the client-side merge (`updateMap`) exists precisely so responses can be partial. When in doubt, return a `Summary` — but only of what was touched.

## The client merge (pattern)

`frontend/src/lib/part.ts` is the reference implementation: an entity class, a `mapableState` collection, and fetch calls that merge the response into it — `parts.updateMap([data])` for a bare entity, `updateSummary(data)` for a `Summary` (`frontend/src/lib/user.ts`). `mapableState` (`frontend/src/lib/mapable.svelte.ts`) is the collection factory: a `$state` record of entities by id with the write operations attached — `setMap` replaces the whole record, `updateMap` merges, `deleteItem` removes. Because the operations are keys of the record, a state collection must not be enumerated with `Object.values`/`filterValues`; use `stateValues`, which skips the operation keys. The full state-object mechanics live in [state-object.md](state-object.md).

## Recipe: add a new mutating operation

1. **Domain** — write the operation as a method on the entity in `backend/domain/src/entities/`. Compute side effects inline in one transaction (through `register`/`Factor` when usage is involved). Return the `Summary` of everything touched, or the bare entity if nothing else is affected. Done when the response covers every entity the operation touched — and nothing it didn't.
2. **Presentation** — a thin axum handler in `backend/axum/src/domain/<module>.rs`: extract session and JSON, call the domain operation, return it. It computes nothing.
3. **Client** — a fetch call in `frontend/src/lib/<name>.ts` that merges the response via `updateSummary` or `updateMap`.
4. **Both stores** — verify the operation on both adapters (the store-seam contract above): run the in-memory suite (`tb_domain`) and the seam suite (`cargo test -p tb_sqlx --test store_seam -- --include-ignored` with a `SCRATCH_DATABASE_URL`) against it, and where the operation is a new path, add it to the seam suite's representative set. Done when both suites pass on the one rule — and where they disagree, fix the in-memory store to the Postgres behavior, the source of truth.

To add the _entity_ itself, follow the implementation steps in [`new-entity.md`](new-entity.md).

## To be reviewed

These asymmetries are filed for review; do not assume they are intended design:

1. The drain loop in `Header.svelte` only continues while `activities` keep arriving — a change without activities waits for the next 60s tick. → [issue #305](https://github.com/hcrohland/tendabike/issues/305)
2. `GET /api/user/summary` piggybacks `update_gear` in the same transaction — every hydration can mutate state. → [issue #306](https://github.com/hcrohland/tendabike/issues/306)
3. The drain loop reads the raw `data["activities"]` array instead of going through the entity layer. → [issue #307](https://github.com/hcrohland/tendabike/issues/307)

## Out of scope here

- The Strava oauth flow: `backend/axum/src/strava/oauth.rs`
- The HTTP session mechanism: `backend/axum/src/strava/session.rs`
- The full route table: `backend/axum/src/domain.rs` plus each module's `router()`
