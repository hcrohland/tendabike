# Adding a New Entity

The implementation steps for a new entity, end to end. *Why* the pieces are wired this way and *what* responses must carry: [`domain-flow.md`](domain-flow.md). Each step is independent: implement it, test it, and move to the next step only when its tests are green. `part` is the reference implementation to copy at every step. How to write and run the tests: `backend/docs/tests/` (`domain.md`, `sqlx.md`, `axum.md`).

## Steps

1. **Domain entity** — `backend/domain/src/entities/<name>.rs` with its mutating operations (recipe in [`domain-flow.md`](domain-flow.md)), plus the `Store` trait methods in `backend/domain/src/traits/`.
   *Test*: the new entity and its operations.
2. **`Summary` field** — add the entity to `Summary` and `SumHash` (`backend/domain/src/entities/summary.rs`) so it flows through every response.
   *Test*: the new entity appears in the `Summary` returned by its operations.
3. **Persistence** — the sqlx implementation of the store methods in `backend/sqlx/src/store/<name>.rs`, with the SQL migration in `backend/sqlx/migrations/` as part of this step.
   *Test*: the new store methods.
4. **Presentation** — the axum router module in `backend/axum/src/domain/<name>.rs` plus the route in `domain.rs`.
   *Test*: the new route.
5. **Frontend class** — the class plus its `mapableState` collection in `frontend/src/lib/<name>.ts` (copy the shape of `part.ts`).
   *Test*: the class and its mapping from the backend JSON.
6. **Summary wiring** — entries in `setSummary`/`updateSummary` in `frontend/src/lib/user.ts`.
   *Test*: the new entity hydrates and merges through `setSummary`/`updateSummary`.

Complete when every step's tests are green, the new entity flows through the `Summary`, and both halves carry tests.
