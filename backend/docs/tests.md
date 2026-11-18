# Backend tests (shared)

Shared conventions for every `tb_*` test suite. Each crate's guide covers its own doubles, fixtures, coverage map, and crate-specific gotchas — read the one for the crate you're touching:

| Crate | Guide |
|-------|-------|
| `tb_domain` | [domain.md](tests/domain.md) |
| `tb_provider` (Strava) | [strava.md](tests/strava.md) |
| `tb_provider` (ICU) | [icu.md](tests/icu.md) |
| `tb_axum` | [axum.md](tests/axum.md) |
| `tb_exec` | [exec.md](tests/exec.md) |
| `tb_sqlx` | [sqlx.md](tests/sqlx.md) |

The run recipe is in [`AGENTS.md`](../AGENTS.md) → *Tests*.

## The seam model

Every suite runs the crate's own logic for real and scripts only its I/O seams, delegating the behaviour it doesn't own to the suite that does. The real-Postgres behavioral bar is `tb_sqlx`'s `#[ignore]`d seam suites; every other suite is DB-free.

| Crate | Owns | Scripts (mocks) | Delegates to |
|-------|------|-----------------|--------------|
| `tb_domain` | domain results (`Summary`, `TbResult`) | the store seam (`MockStore`) | store I/O → `tb_sqlx` |
| `tb_provider` | provider → domain mapping + provider logic | the provider store seam (`MockStravaDataStore` / `MockIcuDataStore`) | store I/O → `tb_sqlx`; domain results → `tb_domain` |
| `tb_exec` | frame-push / orchestration | the loop's seams (`MockConn`, `MockDomainFacade`, `MockLane`) | store I/O → `tb_sqlx`; domain results → `tb_domain`; provider mapping → `tb_provider` |
| `tb_axum` | the HTTP layer (status, JSON, routing, `Error`→response, SSE) | the domain (`MockServiceApi`) | loop + store truth → `tb_sqlx`; dispatch → `tb_domain` |
| `tb_sqlx` | store I/O + the executor's txn lifecycle, on real Postgres | nothing at the store level | domain results → `tb_domain`; orchestration → `tb_exec` |

## Writing tests

- `#[tokio::test]` for async; return `TbResult<()>` so `?` propagates domain errors with context.
- Script only the seam methods the operation under test calls. Every double is stateless: an unscripted call panics — a test bug, not a store/seam bug.
- Canned rows and domain values are struct literals (domain entities have pub fields); the suites have no wall clock, so every fixture time is a fixed value.

## Gotchas

- **mockall 0.15:** an `expect_*()` with no `.times(n)` is satisfied by zero calls — pin `.times(n)` to assert a call happens (or `.times(0)` that it doesn't), else a test passes without the behaviour it gates.

## The shared provider kit

The provider HTTP seams share one canned-response kit, `tb_provider::test_support` (`provider/src/test_support.rs`): `CannedState` plus the `begin_test` / `in_test_mode` / `queue` / `queue_error` / `pop_test_canned` / `requests` free functions. Each driver's seam (`strava/test_support/seam.rs`, `icu/test_support/seam.rs`) is a thin wrapper over it — it owns the per-provider `thread_local` and re-exposes the free functions. The shared scripting surface:

- `queue(uri, json)` — script a JSON response body for an **exact** request URI (FIFO per URI)
- `queue_error(uri, error)` — script an error for an exact URI; unscripted URIs yield `Error::BadRequest`
- `requests() -> Vec<String>` — every requested URI, in order (assert on it to verify API calls)
