# Removing `tb_teststore`: generic-over-domain test re-architecture

**Map #520 · Research ticket #521**

**Question.** Is the following re-architecture feasible, what is its shape, and does it
eliminate the pub bloat + store boilerplate?

> Remove the `tb_teststore` crate, make the stores local to their crates (the
> domain's full `Store` impl lives in `tb_domain`), and make `tb_exec`/`tb_strava`
> **generic over the domain** so they test against a **minimal test domain** —
> starting from `main` (PR #519 / `feature/teststore` is abandoned).

**Bottom line.** **Feasible, and it is the cleaner path.** Starting from `main` the
re-architecture is *purely additive*: the in-crate stores and in-src tests are already
in place (no `tb_teststore`, no pub bloat), so the only new work is the genericity
refactor plus a minimal test domain. It eliminates both the 44-item pub bloat (all
44 → `pub(crate)`) and the store boilerplate (the strava/axum test doubles drop from
~93 methods each to ~18). The shape is a `Domain` trait (associated types + the
entity operations strava/exec drive) that the concrete `tb_domain` and a minimal test
domain both satisfy; `StravaStore`/`Txn`/`TxnSource` become generic over it; the
`StravaStore: Store` supertrait becomes a *combined bound* (associated types cannot be
supertraits). **Generics + associated types + `async_trait` is the idiomatic choice —
no GATs, no trait objects.** All six #510 carry-forward decisions survive the placement
change. Recommend starting from **`main`**.

---

## 0. Starting state (verified on `origin/main`)

- There is **no** `tb_teststore` crate on `main`. Workspace members are
  `app, domain, axum, strava, sqlx, exec` (`Cargo.toml:5-12`).
- The stores are already in-crate:
  - `tb_domain::test_support` — `MemStore` + `mem_*.rs` (the full ~78-method `Store`
    impl), ~3,242 lines (`backend/domain/src/test_support/`).
  - `tb_strava::test_support` — `TestStravaStore` + `TestStravaSession`
    (`backend/strava/src/test_support.rs`, 751 lines).
  - `tb_axum::test_support` — `LiveConn`/`LiveSource` (stream tests) and `FakeConn`/
    `FakeSource` (fast-fail routes) (`backend/axum/src/test_support.rs`).
- The domain tests and strava tests are **in-src** `#[cfg(test)]` modules (one
  `<name>_tests.rs` per entity, `backend/domain/src/entities/*/*_tests.rs`,
  `backend/strava/src/*/*_tests.rs`).
- The load-bearing seams are as stated in the ticket:
  - `StravaStore: tb_domain::Store + Send` — `backend/strava/src/traits.rs:12`.
  - `run<T, S>` with `T: TxnSource + Clone + Send + Sync + 'static, T::Conn:
    StravaStore, S: StravaSession + Send + 'static` —
    `backend/exec/src/executor.rs:325-336`.
  - `TxnSource::Conn: Store + Txn` — `backend/exec/src/txn.rs:47`.
  - `Txn: Store` — `backend/exec/src/txn.rs:61`.
  - `Store` is the marker over nine sub-traits — `backend/domain/src/traits.rs:37`.

So `main` already has the "stores local to their crates" half of the re-architecture.
The new work is the genericity + minimal test domain.

---

## 1. The domain interface `tb_strava` / `tb_exec` consume (Q1)

This is the abstraction surface. It is **not** just the `Store` methods — strava
drives **domain operations on concrete entity types** and **constructs** entity
values. That is the crux of the design.

### 1.1 Direct `tb_domain` references (non-test source)

`grep -rn "tb_domain" backend/{strava,exec}/src` (excluding `*_tests.rs`):

| Crate | File | Items |
|-------|------|-------|
| strava | `traits.rs:9,12` | `Session`, `TbResult`, `UserId`, `tb_domain::Store` |
| strava | `lib.rs:21` | `use tb_domain::*` (wildcard) |
| strava | `event.rs:104,139` | `tb_domain::Error` (as `type Error = …` on two `TryFrom` impls) |
| exec | `txn.rs:32` | `Store`, `TbResult` |
| exec | `executor.rs:38` | `Error`, `Summary`, `TbResult`, `WriteOutcome`, `exec` |
| exec | `message.rs:38` | `ApiWrite`, `Summary`, `TbResult`, `WriteOutcome` |

### 1.2 What strava *calls* on the domain

**Domain `Store` sub-trait methods** (called directly on the store) — exactly **one**:
- `store.partid_get_by_source(&strava_id)` — `backend/strava/src/gear.rs:55,66`.

**Domain operations** (inherent methods / associated fns on concrete `tb_domain`
types) — these are the load-bearing surface:
- `Activity::upsert(user, store)` — via `send_to_tb`,
  `backend/strava/src/activity.rs:161`.
- `ActivityId::new(act).read(user, store)` — `backend/strava/src/activity.rs:185`.
- `ActivityId::new(act).delete(user, store)` — `backend/strava/src/activity.rs:205`.
- `Part::create(name, vendor, model, what, source, purchase, user, store)` —
  `backend/strava/src/gear.rs:74`.
- `PartId::from(gear).part(user, store)` — `backend/strava/src/gear.rs:25`.
- `StravaId::disable(store)` — a *strava-crate* method that calls the store
  (`backend/strava/src/event.rs:376,338`).

**Entity construction** (strava builds domain values by field):
- `into_activity` constructs an `Activity` with **all 14 fields**
  (`backend/strava/src/activity.rs:76-110`); the `Activity` struct has 14 public
  fields (`backend/domain/src/entities/activity.rs`).
- `ActivityId::new`, `PartId::from` — id constructors.

**`StravaStore` methods** (the 15 strava-specific methods, all in the strava crate) —
`backend/strava/src/traits.rs:12-186`: `stravaid_get_user_id`, `strava_event_delete`,
`strava_event_set_time`, `stravaevent_store`, `strava_event_get_next_for_user`,
`strava_event_get_later`, `strava_events_delete_batch`, `stravausers_get_all`,
`stravauser_get_by_tbid`, `stravauser_get_by_stravaid`, `stravauser_new`,
`stravaid_update_token`, `strava_events_get_count_for_user`,
`strava_events_delete_for_user`, `stravauser_delete`.

**Types strava names by value:** `Error`, `TbResult`, `Summary` (`Summary::default()`,
`+=`), `UserId`, `Activity`, `ActivityId`, `ActTypeId`, `PartId`, `PartTypeId`.

### 1.3 What exec *calls* on the domain

- `exec(write, session, store)` — the `ApiWrite` dispatch,
  `backend/domain/src/apiwrite.rs:283` (a ~30-arm `match` over `ApiWrite` variants,
  each arm a domain operation).
- `process(session, store)` — the strava event processing (in `tb_strava`, not the
  domain).
- Types: `Error`, `Summary`, `TbResult`, `WriteOutcome`, `ApiWrite`, `Store`,
  `StravaSession`, `StravaStore`.

The exec **unit tests** (`backend/exec/src/executor_tests.rs`) are *pure-brain* —
"no store fakes, no database" (module doc). They exercise the select/reclaim/backoff/
frame logic with concrete `tb_domain` types and reach into the `Summary` structure
directly: `frame.usages.upsert(usage)`, `frame.usages.tombstone(usage.id)`,
`Summary::default()` (`executor_tests.rs:300-330`). They do **not** drive `run` with a
store; the transaction seam is verified against Postgres in
`backend/sqlx/tests/executor_seam.rs`.

### 1.4 The abstraction surface, in one line

> The `Domain` trait must expose: the **error/summary/session/store** associated
> types, the **entity types strava constructs/deconstructs** (`Activity`, `ActivityId`,
> `PartId`, `Part`, …), and the **operations strava/exec drive**
> (`upsert_activity`, `read_activity`, `delete_activity`, `create_part`,
> `partid_get_by_source`, `exec`). It is an *entity-operation* abstraction, not a
> *store-method* abstraction.

---

## 2. The generic-abstraction shape (Q2)

### 2.1 The `Domain` trait

A trait `Domain` (home: a small seam crate, or `tb_domain` — see §2.5) with:

- **Associated types:** `Error`, `Summary` (`Default + Clone + AddAssign<Summary>`,
  plus an `is_empty()`), `Session`, `Store` (opaque to the generic consumer),
  `ApiWrite`, `WriteOutcome`, and the entity types strava/exec name
  (`Activity`, `ActivityId`, `PartId`, `PartTypeId`, `ActTypeId`, `UserId`, `Part`,
  `Usage`).
- **Methods (async, `async_trait`-boxed):** the operations from §1 —
  `upsert_activity`, `read_activity`, `delete_activity`, `create_part`,
  `partid_get_by_source`, `exec`; plus constructors for the values strava builds
  (`make_activity(…14 fields…)`, `make_activity_id`, `make_part_id`).

The concrete `tb_domain` impls `Domain` (mapping its full `Store` + real operations to
the interface). A **minimal test domain** impls it with stub operations and a tiny
store (§3).

### 2.2 How the supertrait becomes generic

`StravaStore: tb_domain::Store + Send` cannot be written generically as
`StravaStore<D>: D::Store`, because **supertraits must be named traits, not
associated types.** The Rust Reference: supertraits "are declared by trait bounds on
the `Self` type of a trait and transitively the supertraits of the traits declared in
those trait bounds" (https://doc.rust-lang.org/reference/items/traits.html#supertraits)
— i.e. `Self: Shape` where `Shape` is a *named* trait. `D::Store` is an associated
type, so it cannot sit in that position.

The supertrait relationship therefore becomes a **combined bound** at the use sites:

```rust
// before (concrete)                      // after (generic)
pub trait StravaStore: tb_domain::Store    pub trait StravaStore<D: Domain> { … }   // standalone
pub trait Txn: Store                       pub trait Txn<D: Domain> { … }           // standalone
pub trait TxnSource { type Conn: Store+Txn } pub trait TxnSource<D: Domain> { type Conn: D::Store + Txn<D>; … }
```

and the executor's bound tightens to:

```rust
pub async fn run<D: Domain, T, S>(…) where
    T: TxnSource<D> + Clone + Send + Sync + 'static,
    T::Conn: D::Store + StravaStore<D> + Txn<D>,
    S: StravaSession<D> + Send + 'static,
```

This is a mechanical, consistent change (every `T::Conn: StravaStore` / `Txn` bound
gains the `D::Store` half).

### 2.3 GATs vs trait objects — which is idiomatic

**Neither is needed; generics (monomorphization) is the idiomatic choice.**

- **GATs are not required.** Generic Associated Types exist for associated types that
  are *functions of a lifetime or type parameter* (e.g. `type Iter<'a> for &'a Self`)
  (https://doc.rust-lang.org/reference/items/generic-associate-types.html). The
  `Domain` trait's associated types are plain (no lifetime/type-parameter dependence).
  The async methods are handled by `async_trait`, which **boxes** the returned future —
  the same mechanism the codebase already uses for `Store`, `StravaStore`, `Txn`,
  `TxnSource` (`#[async_trait::async_trait]` throughout). No GAT.
- **Trait objects are not idiomatic here.** A `dyn Domain` would (a) require every
  associated type to be named on the object (a trait object's base trait must be
  *dyn-compatible*; associated types must be fixed for the object to be well-formed —
  https://doc.rust-lang.org/reference/types/trait-object.html,
  https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility), and (b)
  switch to **dynamic dispatch** (a vtable of function pointers), losing the concrete
  types strava/exec construct and deconstruct (`Activity`, `Part.id`, …). The whole
  point of the re-architecture is that the generic consumer works with the *concrete*
  domain's types, so monomorphization (static dispatch) is what we want.

The codebase already leans on exactly this machinery — `TxnSource::Conn` (an
associated type) and `Txn: Store` (a supertrait) — so the generic `Domain` is a
natural extension, not a new idiom.

### 2.4 The two-crate-instances root cause (why `tb_teststore` forced `pub`)

The E0277 that forced the strava/domain suites to `tests/` is a direct consequence of
the **orphan rule** and per-instance trait identity: "a trait implementation is only
allowed if either the trait or at least one of the types in the implementation is
defined in the current crate" (https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules).

`tb_teststore` depends on `tb_strava`/`tb_domain` (to impl their traits). When
`tb_strava` takes `tb_teststore` as a **dev-dependency**, the unit-test binary links
`tb_strava` **twice**: once as the lib under test, once as `tb_teststore`'s
dependency. The `StravaStore` trait (defined in `tb_strava`) then exists in **two
copies**; `TestStore`'s `impl StravaStore` (compiled against copy A, in `tb_teststore`)
does not satisfy a `StravaStore` bound on copy B (the lib under test) → E0277 → the
tests must move to `tests/` (an integration test links the lib rlib once) → the items
the tests touch must be `pub`. This is exactly what the `feature/teststore` strava test
doc records: *"They are integration tests … because the store double,
`tb_teststore::TestStore`, depends on `tb_strava` — only an integration test links the
lib rlib once, so `TestStore`'s `StravaStore` impl is the same trait instance the code
under test is bound to (spec #510, amendment)."*
(`origin/feature/teststore:backend/docs/tests/strava.md:3`).

**In-crate stores sidestep this entirely** (§4): the test double and the trait live in
the *same* crate instance, so there is one trait instance and the impl type-checks in
an in-src `#[cfg(test)]` module.

### 2.5 Where the `Domain` trait lives

Dependency direction is `domain ← strava ← exec`. The trait is consumed by both
`strava` and `exec`, and **implemented** by `tb_domain` and by the in-crate test
domains. It cannot live in `tb_strava`/`tb_exec` (then `tb_domain` would have to
depend on `tb_strava`, reversing the arrow). Two defensible homes:

1. **A small seam crate** (e.g. `tb_domain_api`) defining `Domain` (+ the generic
   `StravaStore<D>`/`Txn<D>`/`TxnSource<D>`). `tb_domain`, `tb_strava`, `tb_exec`, and
   the test domains all depend on it. Cleanest isolation; one new tiny crate.
2. **`tb_domain` itself** defines `Domain` and impls it. `tb_strava`/`tb_exec` are
   generic over `tb_domain::Domain`; the test domains (already in crates that depend
   on `tb_domain`) impl it. No new crate, but it puts the abstraction in the concrete
   domain crate.

Both are sound. Option 1 is the more navigable "seam" (the abstraction is its own
module); option 2 is less ceremony. Either way the *shape* is identical.

---

## 3. The minimal test domain + store (Q3)

The minimal test domain is a **stub** `Domain` impl (lives in `tb_strava::test_support`
/ `tb_exec::test_support`, in-crate):

- `Error` — a minimal error type (a few variants: `NotFound`, `DatabaseFailure`,
  `BadRequest`; enough for the strava/exec paths).
- `Summary` — a near-unit type: `Default`, `Clone`, `AddAssign` (no-op merge),
  `is_empty() → true`.
- `Session` — a minimal session (`user_id()`, `shop() → None`, `is_admin()`).
- `Store` — a **tiny** store (see below).
- **Stub operations:** `upsert_activity → Summary::default()`, `read_activity → a
  default `Activity``, `delete_activity → Summary::default()`, `create_part → a default
  `Part``, `partid_get_by_source → None` (or a seeded value), `exec →
  WriteOutcome::Summary(Summary::default())`.

Because the domain operations are **stubs**, the test store's *domain* half is tiny —
the stubs do not call it (except `partid_get_by_source` / `create_part`, which the
strava gear path exercises). So the minimal test store impls:

- the **15 `StravaStore<D>` methods** (event queue + user map — what the strava tests
  actually drive), and
- **2 domain-store methods** (`partid_get_by_source`, `create_part`) for the gear
  path, and
- **`Txn<D>`** `commit`/`rollback` (no-ops, §7).

**≈ 18 methods**, versus the current `TestStravaStore`'s **≈ 93** (15 strava + 78
delegating domain sub-trait methods, `backend/strava/src/test_support.rs:370-744`).
That delta is the "store boilerplate" the re-architecture removes.

For **exec**, the loop `run<D, T, S>` can be driven by a fake `TxnSource<D>` whose
`Conn` is the minimal store (`D::Store + StravaStore<D> + Txn<D>`). The existing exec
unit tests stay pure (they never build a store), so they compile against any
`D: Domain` unchanged in intent.

---

## 4. The domain's in-crate store (Q4)

**Confirmed on `main`:** the domain's full `Store` impl (`MemStore` + `mem_*.rs`,
~78 methods) lives in `tb_domain::test_support`
(`backend/domain/src/test_support/mem_store.rs`, `mem_*.rs`), and the domain tests are
in-src `#[cfg(test)]` modules.

**Why in-crate avoids the two-crate-instances problem.** An in-src `#[cfg(test)]`
module is compiled *into* the same crate as the lib. So `tb_domain`'s unit tests link
`tb_domain` **once**: the `MemStore`'s `impl Store` and the `Store` trait it
implements are the **same trait instance**, and the impl type-checks with no E0277.
No external crate depends on `tb_domain` for the test double, so there is no second
link of `tb_domain`, and nothing is forced `pub` — the 35 domain items the tests touch
stay `pub(crate)`.

Contrast with `tb_teststore` (§2.4): there the double lived in a *separate* crate that
depended on `tb_domain`, forcing a second link of `tb_domain` in the test binary and
the E0277 that pushed the suites to `tests/` and the items to `pub`.

The in-crate store is therefore the structural reason the domain half needs no pub
widening at all.

---

## 5. All 44 items → `pub(crate)` (Q5)

The `feature/teststore` branch forced **44** production items to `pub`
(`git diff main..origin/feature/teststore`): **9 in `tb_strava`**, **~35 in
`tb_domain`**.

- **Strava (9) → `pub(crate)` via the minimal test domain.** With no `tb_teststore`
  dev-dependency and the test double in-crate (`tb_strava::test_support`), the strava
  tests are in-src → one crate instance → the 9 items
  (`StravaActivity`, `into_activity`, `get_type`, `delete_activity`, `rate_limit`,
  `into_partid`, `get_time`, `disable`, `disabled`) revert to `pub(crate)`.
- **Domain (35) → `pub(crate)` via the in-crate store.** The domain tests are in-src
  against the in-crate `MemStore` (§4) → the 35 items
  (`Factor`, `read_optional`, `usage`, `find`, `register`, `read`, `dispose`,
  `detach_assembly`, `subparts`, `calculate_usage`, `recalculate`, `check_owner`, …)
  stay `pub(crate)`.

**All 44 → `pub(crate)`.** No public-API widening.

---

## 6. The sqlx seeder + axum stream tests (Q6)

### 6.1 sqlx scratch seeder — unchanged

`backend/sqlx/tests/common/scratch.rs` seeds the scratch Postgres database from
`tb_domain::test_support::{MemStore, StoreSnapshot}`:
`let snap = MemStore::prepopulated().snapshot();` (`scratch.rs:220`), then raw `INSERT`s
for users/parts and the domain store traits for the rest (`load_fixture`,
`scratch.rs:250`). **The snapshot generator lives in the in-crate domain store:**
`build_workshop_store()` (`backend/domain/src/test_support/mem_store.rs:119`) builds the
workshop through domain operations; the checked-in `SNAPSHOT_JSON`
(`backend/domain/src/test_support/prepopulated_data.rs:7`) is the static fixture; the
regeneration bin is `backend/domain/src/bin/build_snapshot.rs`.

Because `tb_sqlx` depends on the **concrete** `tb_domain` (the re-architecture only
generics `strava`/`exec`, not `domain`), and the seeder uses the in-crate `MemStore`,
**the sqlx seeder is unaffected.** No `tb_teststore` is involved. The snapshot
generator stays in `tb_domain::test_support`.

### 6.2 axum stream tests — re-implemented over the minimal domain, split preserved

The stream tests run the **real** `tb_exec` loop, which needs a working `TxnSource`
whose `Conn` is a full working store. On `main` that is `LiveConn`/`LiveSource`
(`backend/axum/src/test_support.rs`): `LiveConn` wraps a `MemStore` + a shared
`StravaState` + the `panic_next` arm, and impls all nine `Store` sub-traits
(delegating to `mem`), `StravaStore` (the queue + the panic arm in
`strava_event_get_next_for_user`), and `Txn` (`commit`/`rollback` delegate to
`mem.commit`/`mem.rollback`). `LiveSource` is the `TxnSource` whose `begin()` returns a
`LiveConn` (a sibling transaction on the shared `Arc<Mutex<MemStore>>`).

Under the re-architecture these are re-implemented **over the minimal test domain**:

- `LiveConn` impls `D::Store` (minimal) + `StravaStore<D>` (queue + panic arm) +
  `Txn<D>` (no-op/simple commit). The **nine `Store` sub-traits (78 delegating methods)
  collapse into the minimal `D::Store`** — the stub domain operations do not need them.
- `LiveSource` stays the `TxnSource<D>`; `begin()` returns the minimal `LiveConn`.
- **The store/source split is preserved** — it is inherent to the `TxnSource`/`Txn`
  seam (a source mints connections; a connection is a store + a transaction). That
  split is an *overhead consideration* (one extra type + the `begin()`), not a
  correctness issue; it is unchanged by genericity.
- The `FakeConn`/`FakeSource` (fast-fail routes, `begin()` → `DatabaseFailure`) shrink
  the same way: `StravaStore<D>` (15 `unimplemented!()`s) + `Txn<D>` + a minimal
  `D::Store`, instead of ~93 `unimplemented!()` methods
  (`backend/axum/src/test_support.rs:150-560`).

So the "full working store" the stream tests need is satisfied by the **minimal**
domain's store (stub operations + the strava queue + the no-op transaction lifecycle),
and the axum test doubles drop from ~93 methods each to ~18.

---

## 7. Carry-forward #510 decisions (Q7)

The `feature/teststore` `TestStore` (`origin/feature/teststore:backend/teststore/src/lib.rs`)
encodes the #510 decisions. Their status under the new **in-crate + generic**
structure:

| #510 decision | Where it lives today | New structure | Status |
|---|---|---|---|
| **No transaction buffer / no-op `Txn` commit+rollback** | `TestStore`: "no transaction buffer: every write lands directly in the shared state"; `Txn` "consuming, no-op commit/rollback" (`lib.rs:1-20`) | The minimal test domain's store has no buffer; `Txn<D>` commit/rollback are no-ops | **Carries forward** |
| **Six buffer-discipline tests stay removed** | Removed with the buffer (they tested commit/abort discipline the buffer provided) | No buffer in the minimal test domain → the six tests stay removed | **Carries forward** |
| **`Arc<Mutex<…>>` lock** | `TestStore.state: Arc<Mutex<TestStoreState>>` (`lib.rs:44`) | The minimal test domain's shared state is `Arc<Mutex<…>>` (the axum stream tests need the executor task + test thread to share it) | **Carries forward** |
| **The panic arm** | `TestStore.panic: Arc<AtomicBool>` + `arm_panic()` (`lib.rs:46,14`); `LiveConn::strava_event_get_next_for_user` panics when armed (`backend/axum/src/test_support.rs`, `main`) | The minimal test domain's strava-queue read keeps the `panic: Arc<AtomicBool>` seam (spec §4.6 panic teardown) | **Carries forward** |
| **Retired `Fault`/`fail_next`** | Absent from `TestStore` (it was in `main`'s `MemStore`: `backend/domain/src/test_support.rs:232,322`) | The minimal test domain (strava/exec) has **no** fault injection. **Note:** the domain's in-crate `MemStore` (for *domain* tests) keeps `Fault`/`fail_next` — that is a pre-existing, separate `main` feature the domain tests use, not part of the strava/exec test store | **Carries forward** (scoped to the minimal test domain) |
| **The boundary rule** | Store-seam contract: domain ops are bounded by the `Store` marker (not `Txn`), verifiable on both `MemStore` and `SqlxConn`, Postgres as source of truth (`backend/domain/src/apiwrite.rs:34-40` module doc) | Unchanged — the re-architecture does not move the domain/store boundary | **Carries forward** |

**Catalog:** all six #510 decisions **carry forward**; **none are re-opened** by the
placement change. The only nuance is scoping: "retired `Fault`/`fail_next`" applies to
the **minimal test domain** (the strava/exec double). The domain's in-crate `MemStore`
retains its fault injection as a pre-existing feature for the domain's own in-src tests.

---

## 8. Starting-point analysis (Q8)

`origin/main` is one commit ahead of `feature/teststore`'s merge-base
(`b937f4b` "Rewrite small-increments rule"); `feature/teststore` is 8 commits of
teststore work on top of the older base. `feature/teststore` vs `main`: **77 files,
+1,996 / −2,968**.

| | Start from **`main`** | Start from **`feature/teststore`** |
|---|---|---|
| In-crate stores | Already present (keep) | Must **re-create** (the `test_support/` dirs were deleted; `MemStore` moved to `tb_teststore`) |
| In-src tests | Already present (keep) | Must **move back** from `tests/` (15 domain + 4 strava files) |
| Pub bloat | None (items are `pub(crate)`) | Must **revert** the 44 `pub` → `pub(crate)` |
| `tb_teststore` crate | Absent (do not add) | Must **delete** |
| New work (both) | The genericity refactor + `Domain` trait + minimal test domain + generic `StravaStore<D>`/`Txn<D>`/`TxnSource<D>` + axum doubles over the minimal domain | Same, **plus** all the reversals above |
| Net churn | **Additive only** | Additive **+** a large reversal (undo the crate, the test relocation, the pub widening) |

**`main` is the cleaner path.** From `main` the re-architecture is purely additive:
the in-crate stores and in-src tests (and the `pub(crate)` visibility) are already the
goal state, so the work is exactly the genericity + minimal test domain. From
`feature/teststore` you first undo the `tb_teststore` crate, the `tests/` relocation,
and the 44-item pub widening, *then* do the same additive work — strictly more churn
and more risk for no benefit. **Recommendation: start from `main`.**

---

## 9. Churn / risk / idiomaticity (Q9)

**Effort (from `main`): moderate, mechanical.**
- New: the `Domain` trait (associated types + ~6-8 async ops + constructors), the
  minimal test domain (stub ops + ~18-method store), the generic
  `StravaStore<D>`/`Txn<D>`/`TxnSource<D>`, and re-implementing the axum
  `LiveConn`/`FakeConn` over the minimal domain.
- Touched: every `tb_strava`/`tb_exec` signature that names a domain type gains a
  `D: Domain` parameter and `D::` prefixes; the `run`/`queue_probe`/`run_write`/
  `run_event` bounds gain `D::Store`.

**Biggest risks (ranked):**
1. **The `Domain` trait surface is the hard part** — not the store methods, but the
   *entity-operation* surface: strava **constructs** `Activity` (14 fields) and reads
   `Part.id`, and the exec tests reach into `Summary` (`frame.usages`). The trait must
   expose constructors + accessors for these, which is the fiddliest design decision.
   Mitigation: keep the entity types as associated types with a small set of
   constructors/accessors; the stub test domain makes the required surface obvious.
2. **Genericity boilerplate** — every strava/exec function signature changes.
   Mechanical, reviewable, low-logic-risk; the main cost is volume.
3. **Supertrait → combined bound** — `StravaStore: Store` / `Txn: Store` become
   `D::Store + StravaStore<D>` / `D::Store + Txn<D>` at every use site. Must be applied
   consistently; a missed site is a compile error (loud, not silent).
4. **Monomorphization** — the generic code instantiates per `D` (real domain + test
   domain). No runtime cost; a marginally larger test binary. Not a real risk.
5. **`async_trait` interaction** — the `Domain` ops are `async_trait`-boxed futures,
   consistent with the existing `Store`/`StravaStore`/`Txn`/`TxnSource`. No new
   mechanism.

**Idiomaticity: high.** Generics + associated types is Rust's standard abstraction
mechanism (the codebase already uses `TxnSource::Conn` and `Txn: Store`); `async_trait`
is the standard async-trait approach (already used throughout); no GATs and no trait
objects are needed. The `Domain` trait is a textbook "ports" abstraction for
testability. This is idiomatic Rust, not a fight against the type system.

---

## 10. Ranked recommendation

| Rank | Option | Bar met | Churn | Idiomaticity |
|------|--------|---------|-------|--------------|
| **1** | **Start from `main`; add the `Domain` trait + minimal test domain; make `strava`/`exec` generic; keep the in-crate domain store** | **Yes** — removes `tb_teststore` (not re-added), keeps stores in-crate, all 44 → `pub(crate)`, test doubles ~93 → ~18 methods, all six #510 decisions preserved | **Lowest** — purely additive on `main` | **High** — generics + assoc types + `async_trait`; no GATs, no trait objects |
| 2 | Start from `feature/teststore`; reverse the crate + test relocation + pub widening, then add the same genericity | Yes | Higher — a large reversal before the additive work | High (same end state) |
| 3 | Keep `feature/teststore` as-is (external `tb_teststore`, `tests/` suites, 44 `pub`) | No — retains the pub bloat the owner deems "very severe" and the store boilerplate | N/A | Lower — the two-crate-instances workaround is a known anti-pattern |

**Recommendation: Option 1.** The re-architecture is **feasible** and **idiomatic**;
starting from **`main`** it is the **lowest-churn** path and it **eliminates both the
pub bloat (all 44 → `pub(crate)`) and the store boilerplate (test doubles ~93 → ~18
methods)** while preserving every #510 carry-forward decision. The central design
artifact is the `Domain` trait (an *entity-operation* abstraction, home in a small
seam crate or in `tb_domain`); the load-bearing risk is getting its surface right
(the entity constructors/accessors strava/exec use), which the stub test domain makes
concrete and checkable.

---

## Sources

**Codebase (`origin/main`, branch `research/remove-teststore`):**
- `StravaStore: tb_domain::Store + Send` — `backend/strava/src/traits.rs:12`.
- `Store` marker over nine sub-traits — `backend/domain/src/traits.rs:37`; sub-traits
  in `backend/domain/src/traits/{part,user,shop,activity,attachment,partnote,usage,service,serviceplan}.rs`.
- `run<T,S>` bounds — `backend/exec/src/executor.rs:325-336`; `TxnSource::Conn: Store+Txn`
  — `backend/exec/src/txn.rs:47`; `Txn: Store` — `backend/exec/src/txn.rs:61`.
- `exec` dispatch — `backend/domain/src/apiwrite.rs:283`; store-seam boundary rule —
  `backend/domain/src/apiwrite.rs:34-40`.
- strava domain calls — `backend/strava/src/activity.rs:161,185,205`;
  `backend/strava/src/gear.rs:25,55,66,74`; `backend/strava/src/event.rs:338,376`.
- `Activity` 14 fields — `backend/domain/src/entities/activity.rs`; `Summary`
  (`Default`/`AddAssign`/`is_empty`) — `backend/domain/src/summary.rs`.
- In-crate stores — `backend/domain/src/test_support/` (`MemStore`,
  `build_workshop_store` at `mem_store.rs:119`, `prepopulated` at `mem_store.rs:27`,
  `snapshot` at `mem_store.rs:100`, `SNAPSHOT_JSON` at
  `test_support/prepopulated_data.rs:7`, `Fault`/`fail_next` at
  `test_support.rs:232,322`); `backend/strava/src/test_support.rs`
  (`TestStravaStore`, delegating domain impls at `:370-744`);
  `backend/axum/src/test_support.rs` (`FakeConn` `:150-560`, `LiveConn`/`LiveSource`,
  panic arm in `strava_event_get_next_for_user`).
- Snapshot regeneration bin — `backend/domain/src/bin/build_snapshot.rs`.
- sqlx seeder — `backend/sqlx/tests/common/scratch.rs:220,250`.
- exec pure-brain tests — `backend/exec/src/executor_tests.rs` (module doc;
  `Summary` reach-in at `:300-330`).
- `feature/teststore` (abandoned): `backend/teststore/src/lib.rs` (`TestStore`,
  #510 decisions); `backend/docs/tests/strava.md:3` (the two-instance rationale);
  pub-bloat diff `git diff main..origin/feature/teststore` (9 strava + ~35 domain `pub`
  items; 77 files, +1,996/−2,968).

**Rust Reference (primary):**
- Supertraits (named-trait bounds on `Self`; cannot be associated types) —
  https://doc.rust-lang.org/reference/items/traits.html#supertraits
- Dyn compatibility (trait-object restrictions: associated types, dispatchable fns) —
  https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility
- Trait objects (opaque value, vtable / dynamic dispatch) —
  https://doc.rust-lang.org/reference/types/trait-object.html
- Generic Associated Types (associated types parameterized by lifetime/type) —
  https://doc.rust-lang.org/reference/items/generic-associate-types.html
- Orphan rules / coherence (impl allowed only if trait or a type is local; the
  basis of the two-crate-instances E0277) —
  https://doc.rust-lang.org/reference/items/implementations.html#orphan-rules
