# tb_sqlx tests

`SQLX_OFFLINE=true cargo test -p tb_sqlx`. All tests are pure unit tests (no database, no network), colocated in each module's `#[cfg(test)] mod tests`: `lib.rs`, `tb_sqlx.rs`, `stravastore.rs`, and `store/{user,part,activity,attachment,service,serviceplan,shop,usage}.rs`.

## Coverage

- **Helpers** (`lib.rs`): `into_domain` maps `RowNotFound` → `Error::NotFound` and other sqlx errors → `Error::DatabaseFailure`; `vec_into` / `option_into` element mapping
- **Pool** (`tb_sqlx.rs`): `DbPool::lazy` panics on an invalid URL and builds a lazy pool with a 1-second acquire timeout
- **Entity mapping** (`store/*.rs`, `stravastore.rs`): domain → `Db*` field mapping (i32 IDs, UUIDs, JSON `updates`), `Db*` → domain roundtrips via `From` / `TryFrom`, and `#[should_panic]` tests documenting `unwrap()` behavior on unknown onboarding status, unknown object/aspect type, and invalid webhook `updates` JSON

## Gotchas

- All domain entities have **pub fields** — tests construct them with struct literals; no extra dev-deps are needed.
- `DbPool::lazy` (sqlx `connect_lazy`) requires a Tokio context — that one test is `#[tokio::test]` (tokio is already a dev-dependency).
- `oauth2::RefreshToken` does not implement `PartialEq` — assert on `secret()` / `is_none()` instead.
- UUIDs in tests are fixed `Uuid::from_str(...)` values.
- Tests document current behavior as-is (including the `unwrap()` panics on unknown enum strings) — fixing the underlying behavior is a follow-up.
