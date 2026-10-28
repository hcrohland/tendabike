# tb_axum tests

All tests run in-memory (no database; the DB boundary is a fast-failing source, not a real connection). Unit tests are colocated in each module's `#[cfg(test)] mod tests` (`error.rs`, `strava/oauth.rs`, `strava/webhook.rs`, `strava/session.rs`, `domain/partnote.rs`); router tests live in `lib_tests.rs` (the `#[cfg(test)]` module declared in `lib.rs`) and exercise the real router built by the private `routes()` helper.

## Coverage

- **Error mapping** (`error.rs`): every `Error` variant → expected HTTP status + body text through `AppError::into_response()`
- **OAuth helpers** (`strava/oauth.rs`): HMAC-SHA1 known-answer vector for `hmac_signature()`, `gentoken`/`getpath` roundtrip, corrupted-signature and malformed-state rejection
- **Webhook validation** (`strava/webhook.rs`): `Hub::validate()` accepts a valid subscribe, rejects wrong token / unknown mode
- **Session expiry** (`strava/session.rs`): `is_expired()` for past / future / `None` expiry
- **Partnote MIME** (`domain/partnote.rs`): file-extension → MIME type helper
- **Router boundaries** (`lib_tests.rs`, `test_app` fixture): public endpoints (`/api/types/*`, `/strava/callback`, `/strava/login`, `/strava/token`, `/strava/logout`), 401 on unauthenticated `/api/*`, 404 (hidden) on admin-only routes for non-admins, 500 when an authenticated request reaches the DB layer, 405 wrong method, 404 unknown path
- **Executor and stream** (`lib_tests.rs`, `test_app_live` fixture): SSE `event-stream` + heartbeats, summary frames forwarded to the stream, an attached stream keeping the executor alive, writes riding the executor (201 with the created entity, 204 otherwise), onboarding sync / postpone, admin sync consuming the Strava queue, webhook wake draining the queue before the idle timeout, a wake never spawning a reaped executor, executor reap and respawn (including panic → dead → respawn), admin rescan answering 204 and stopping live streams, the hooks route being gone, and the full cutover scenario (write rides the stream, hooks gone)

## Key infrastructure

All in `lib_tests.rs` (the `#[cfg(test)]` module declared in `lib.rs`).

- **`MockServiceApi`** (`tb_exec`) — the `mockall` service-API double the whole suite runs against: no loop, no in-memory store, no executor tokio plumbing. The mock's expectations (`.times(n)` + arg predicates) verify the handler called the right operation with the right args; the canned replies are the mock's; the error mapping (404/500/401/403/400/409/429) is the mock returning `Err(...)` directly — no failing `begin` anywhere.
- **`test_app(store, api)`** — full production `Router` from `routes()` + `SessionManagerLayer` (tower-sessions `MemoryStore` sessions) + `AppState::new(api)`.
- **`run` / `run_json` / `run_raw`** — `tower::ServiceExt::oneshot` roundtrip; returns `(StatusCode, HeaderMap, Vec<u8>)`. `run_raw` is the multipart/raw-body helper (raw bytes plus an explicit content type — `run_json` cannot carry one), and `run`/`run_json` are refactored on top of it.
- **`run_sse` / `read_sse_frames`** — GET a long-lived SSE endpoint and keep the body open (collecting it would hang); read frames with a per-frame timeout.
- **`user_cookie(store)` / `admin_cookie(store)`** — create a session via `RequestSession::new_dummy(is_admin)`, return the `id` cookie header.

What this suite deliberately does not cover: loop + real-store truth (reads-as-messages, FIFO, rollback, reap, queue drain, Registry-level lifecycle, the direct ops' transactions) lives in `sqlx/tests/executor_seam.rs` on real Postgres; the read/write dispatch logic lives in the domain suite (`apiread_tests` beside `apiwrite_tests`).
- **`RequestSession::new_dummy(is_admin)`** — `#[cfg(test)]` constructor in `strava/session.rs` (fields are private; always construct sessions this way in tests).
- **`CANNED_STRAVA` / `canned_strava(uri, json)`** — test-only hook in the production module `strava/session.rs`: maps a Strava URI to a canned JSON response, and `RequestSession::request_json` answers with the canned JSON instead of hitting the Strava API. The `LiveSource` suite runs the real executor loop with a real `RequestSession`, and the summary route's hydration (spec #558 §4) calls `/athlete`: `LiveSource::new` maps an empty athlete so the hydration succeeds offline — no new gear, the common case (no frame, unchanged snapshot).
- Tests force the `CLIENT_ID` / `CLIENT_SECRET` env vars via a `Once` (edition 2024: `std::env::set_var` is `unsafe`), overriding any values from the environment so the redirect-URL assertions are deterministic; `CSRF_SECRET` is left unset (the `CSRF_KEY` falls back to a per-process random key, which is fine for token roundtrips).

## Gotchas

- Auth-gated and admin-gated routes are asserted by status alone (401 `Please login` / 404) — on the `test_app` fixture, anything past the extractors hits `FakeSource::begin`, fails with a `DatabaseFailure`, and yields 500, so don't assert deeper behavior there; use the `test_app_live` fixture for behavior beyond the gate.
- The concrete database never enters `tb_axum`: `start()` receives `tb_sqlx::DbPool` from the composition root (`app/src/main.rs` builds `DbPool::new(&database_url)` and passes it in), and the tests substitute `FakeSource`/`LiveSource` at the same `TxnSource` seam.
