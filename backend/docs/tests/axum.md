# tb_axum tests

`SQLX_OFFLINE=true cargo test -p tb_axum`. All tests run in-memory (no database; the one DB-boundary test attempts a refused TCP connect). Unit tests are colocated in each module's `#[cfg(test)] mod tests` (`error.rs`, `strava/oauth.rs`, `strava/webhook.rs`, `strava/session.rs`); router tests live in `lib.rs` `mod tests` and exercise the real router built by the private `routes()` helper.

## Coverage

- **Error mapping** (`error.rs`): every `Error` variant → expected HTTP status + body text through `AppError::into_response()`
- **OAuth helpers** (`strava/oauth.rs`): HMAC-SHA1 known-answer vector for `hmac_signature()`, `gentoken`/`getpath` roundtrip, corrupted-signature and malformed-state rejection
- **Webhook validation** (`strava/webhook.rs`): `Hub::validate()` accepts a valid subscribe, rejects wrong token / unknown mode
- **Session expiry** (`strava/session.rs`): `is_expired()` for past / future / `None` expiry
- **Router** (`lib.rs`): public endpoints (`/api/types/*`, `/strava/callback`, `/strava/login`, `/strava/token`, `/strava/logout`), 401 on unauthenticated `/api/*`, 404 (hidden) on admin-only routes for non-admins, 500 when an authenticated request reaches the DB layer, 405 wrong method, 404 unknown path

## Key infrastructure

- **`test_support`** (`axum/src/test_support.rs`):
  - `test_app(session_store)` — full `Router` from `routes()` + `SessionManagerLayer` (mirrors `start()` minus Postgres); the DB pool is a lazy `DbPool::lazy("postgres://127.0.0.1:1")` that never connects (port 1 is never listening, errors fail fast)
  - `run(app, method, uri, cookie)` — `tower::ServiceExt::oneshot` roundtrip; returns `(StatusCode, HeaderMap, Vec<u8>)`
  - `user_cookie(store)` / `admin_cookie(store)` — create a session in the store via `RequestSession::new_dummy(is_admin)`, return the `id` cookie as a header (cookie value is the session `Id`'s base64url encoding)
- **`RequestSession::new_dummy(is_admin)`** — `#[cfg(test)]` constructor in `strava/session.rs` (fields are private; always construct sessions this way in tests).
- Tests force the `CLIENT_ID` / `CLIENT_SECRET` env vars via a `Once` (edition 2024: `std::env::set_var` is `unsafe`), overriding any values from the environment so the redirect-URL assertions are deterministic; `CSRF_SECRET` is left unset (the `CSRF_KEY` falls back to a per-process random key, which is fine for token roundtrips).

## Gotchas

- Auth-gated and admin-gated routes are asserted by status alone (401 `Please login` / 404) — anything past the extractors hits the lazy pool and yields 500, so don't assert deeper behavior without a real database.
- `DbPool::lazy()` lives in the `tb_sqlx` crate but is only exercised by tests; `start()` still uses `DbPool::new()` (connect + migrate).
