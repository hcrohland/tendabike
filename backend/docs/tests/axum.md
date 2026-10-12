# tb_axum tests

`SQLX_OFFLINE=true cargo test -p tb_axum`. All tests run in-memory (no database, no network). Unit tests are colocated in each module's `#[cfg(test)] mod tests` (`error.rs`, `strava/oauth.rs`, `strava/webhook.rs`, `strava/session.rs`); router and stream tests live in `lib_tests.rs` (the `#[cfg(test)]` module declared in `lib.rs`) and exercise the real router built by the private `routes()` helper.

## Coverage

- **Error mapping** (`error.rs`): every `Error` variant → expected HTTP status + body text through `AppError::into_response()`
- **OAuth helpers** (`strava/oauth.rs`): HMAC-SHA1 known-answer vector for `hmac_signature()`, `gentoken`/`getpath` roundtrip, corrupted-signature and malformed-state rejection
- **Webhook validation** (`strava/webhook.rs`): `Hub::validate()` accepts a valid subscribe, rejects wrong token / unknown mode
- **Session expiry** (`strava/session.rs`): `is_expired()` for past / future / `None` expiry
- **Router** (`lib_tests.rs`): public endpoints (`/api/types/*`, `/strava/callback`, `/strava/login`, `/strava/token`, `/strava/logout`), 401 on unauthenticated `/api/*`, 404 (hidden) on admin-only routes for non-admins, 500 when an authenticated request reaches the DB layer, 405 wrong method, 404 unknown path
- **Stream/SSE** (`lib_tests.rs`): the `GET /api/user/stream` endpoint with the **real `tb_exec` loop** running — `text/event-stream` with `: ping` heartbeats, `data:` frames carrying the write's `Summary` JSON, the executor lifecycle (an attached stream keeps the executor alive past the idle window; a write-only spawn reaps; a panicked executor — armed with `arm_panic()` — records `Dead` and the next write respawns), executor-backed write handlers (`POST/PUT/DELETE /api/part`, notes, shop subscriptions, onboarding sync/postpone, admin `GET /strava/sync/{id}`, `POST /api/activ/descend`, the admin rescan's `stop_all`), and the webhook's wake signal draining the Strava queue before the idle timeout

## Key infrastructure

- **`test_support`** (`axum/src/test_support.rs`):
  - `test_app(session_store)` — full `Router` from `routes()` + `SessionManagerLayer` (mirrors `start()` minus Postgres); the state carries a `FakeSource` whose `begin` fails fast with a `DatabaseFailure`, so any handler that opens a transaction answers a 500 without reaching a database (its `FakeConn` is `unimplemented!()` — never reached)
  - `test_app_live(session_store, store, registry)` — the same stack, but with the unified in-memory store (`tb_teststore::TestStore`) as the `TxnSource`: the stream endpoint spawns the real `tb_exec` loop, and the executor task and the test thread share the store's state (the store's `Arc`-sharing exists exactly for this). Takes an explicit executor `Registry` so the tests set a short idle/heartbeat cadence and the executor's lifecycle is observable
  - **Per-user seeding** — lives in the test layer (an operation through the store, not store internals): `test_app_live` begins a sibling transaction, creates the first domain user (id 1 — matching `user_cookie`'s dummy session), inserts the `StravaUser` row for user 1 (the executor's session read, `stravauser_get_by_tbid`, needs it), and commits. The row carries **no refresh token**, so the Strava path fails with `NotAuth` and the queue is cleared — the tests' no-auth seam
  - `run(app, method, uri, cookie)` — `tower::ServiceExt::oneshot` roundtrip; returns `(StatusCode, HeaderMap, Vec<u8>)`
  - `run_sse(app, uri, cookie)` / `read_sse_frames(body, count, timeout)` — open a long-lived (SSE) endpoint and read its body frame by frame (a live stream never ends, so the read is bounded by a per-frame timeout)
  - `user_cookie(store)` / `admin_cookie(store)` — create a session in the store via `RequestSession::new_dummy(is_admin)`, return the `id` cookie as a header (cookie value is the session `Id`'s base64url encoding)
- **`RequestSession::new_dummy(is_admin)`** — `#[cfg(test)]` constructor in `strava/session.rs` (fields are private; always construct sessions this way in tests).
- Tests force the `CLIENT_ID` / `CLIENT_SECRET` env vars via a `Once` (edition 2024: `std::env::set_var` is `unsafe`), overriding any values from the environment so the redirect-URL assertions are deterministic; `CSRF_SECRET` is left unset (the `CSRF_KEY` falls back to a per-process random key, which is fine for token roundtrips).

## Gotchas

- Auth-gated and admin-gated routes are asserted by status alone (401 `Please login` / 404) — anything past the extractors hits the `FakeSource`'s failing `begin` and yields 500, so don't assert deeper behavior on the `test_app` stack.
- Stream tests assert on the shared store through `store.state()` (a clone of the shared state) and seed it through operations (`store.begin().await` → write → `commit()`); `TestStore` has no public raw mutator.
- The stream seam has no refresh token: a queued Strava sync is consumed by `NotAuth` disabling the user and clearing the queue, so an **empty queue is consumption** (the `assert_queue_drained` helper).
