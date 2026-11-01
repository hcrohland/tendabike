# ADR-0010: The entity owns its store

## Status

Accepted.

## Context

The domain layer is storage-agnostic: every operation goes through the store traits in `backend/domain/src/traits/`, and the Postgres store (`SqlxConn`, `backend/sqlx/src/store/`) is the production adapter (see `domain-flow.md`, "The store seam"). Each entity already keeps its own CRUD in its own file — `Part` in `entities/part.rs`, `User` in `entities/user.rs`, and so on — and the web layer (`tb_axum`) never touches a store.

But the rule was not written down, and it was not followed everywhere. A store method (named `<entity>_<op>`, e.g. `part_get_all_for_userid`, `notification_create`) could be called from anywhere that held a store: the `ApiWrite` dispatch, the `exec` loop, and — most often — *another* entity. The leaks clustered in three places:

- **Cross-entity**: `User::delete` cascade-deleted a user's data by calling six foreign stores (`services_delete`, `serviceplans_delete`, …) directly; `User::get_summary` read `notification_all_for_user` and `partnote_all_by_part` straight off the store; `Shop` created notifications via `notification_create` and moved parts between shops via `parts_register_shop`/`parts_unregister_shop`/`shop_get_parts`.
- **The `exec` crate**: `Registry::subscribe_shop` read a shop with `shop_get`; the write loop resolved a part's owner with `partid_get_part`.
- **The `ApiWrite` dispatch**: the subscription arms read `subscription_get`; the onboarding arms wrote `update_onboarding_status`.

A future contributor could not tell, from the code alone, which store calls were an entity reaching into its own table and which were a layer (or a foreign entity) reaching across a boundary. The boundary existed in practice for most calls — `get_summary` already routed the bulk of its cross-entity reads through entity methods (`Activity::get_all`, `Attachment::for_part_with_usage`, …) — but the exceptions were indistinguishable from the rule.

## Decision

**Only the entity that owns a store may access it.** A store method (`<entity>_<op>`) is called only from its owning entity's code. When operation X of entity A needs data from entity B, A calls an entity method on B — never B's store.

The rule applies to **every** store, including `StravaDataStore` (`strava_users` / `strava_events`), whose owning "entities" are `tb_strava`'s own `StravaUser` and `Event` code (it was already compliant: the `exec` crate calls `tb_strava`'s entity functions, never the store directly).

Cross-entity needs are served by the callee exposing the operation it owns, and the caller invoking that entity method:

- **Bulk deletion** is by collection, not by user: each entity gains a `delete_many(&[T])` (and `Attachment::delete_by_parts(&[Part])`) wrapping its existing `*_delete` store call. `User::delete` already collects the entities via `get_summary`, then calls these in FK-safe order. No new queries, no per-row re-read.
- **Shop ↔ part** moves go through the Part entity: `Part::for_shop`, `Part::register_to_shop`, `Part::unregister_from_shop`.
- **Notification creation** is a single door: `Notification::create(recipient, lifecycle, kind, subject, message)` — all seven `Shop` call sites become `Notification::create(…)`.
- **Session-less summary reads**: `get_summary`/`get_part_summary` have no acting session, and the data is the user's own, so the reads that leaked are promoted to session-less entity methods — `Notification::all_for_user`, `PartNote::all_for_part` (contrast `PartId::notes`, which takes a session for the ownership check).
- **Non-entity callers** route through an existing or purpose-built entity method: the dispatch's `subscription_get` → `SubscriptionId::read`, `update_onboarding_status` → `UserId::set_onboarding_status`; `exec`'s `shop_get` → `ShopId::read`, `partid_get_part` → `PartId::owner` (a new session-less, check-less read, because the loop is routing a frame, not authorizing one).

**Exemptions** — these are not "accesses" of a store in the rule's sense: the sqlx store files (they *are* the store, they implement it), the `MockStore` double (its test double), the test and seam suites (they drive the store on purpose), and `build_snapshot.rs` (a composition-root tool that seeds the prepopulated snapshot).

## Rejected alternatives

- **Exempt an aggregate root.** Let `User` — the aggregate root that owns the `Summary` — cascade-delete directly, routing only the other leaks through entity methods. Rejected: it carves a named hole in an otherwise-uniform rule, and the bulk-by-collection `delete_many` methods are small and reuse the existing store calls, so the exemption buys little.
- **A domain-service composition point.** Introduce a use-case function allowed to touch all stores for multi-entity operations. Rejected: it reintroduces exactly the "a non-entity layer reaches across stores" shape the rule removes, and there is no single such point — the leaks are spread across `User`, `Shop`, the dispatch, and `exec`.
- **`delete_all_for_user` per entity.** Each entity finds-and-deletes its own rows for a user id. Rejected: it duplicates the `get_summary` read (the cascade already has the rows) and scatters the FK-ordering logic across entities, versus the single ordered call site it replaces.

## Consequences

- The store seam reads uniformly: a store method is called only from its owning entity (plus the exempted locations), so a reviewer can see the boundary at a glance.
- Thirteen entity methods are added (`Part::delete_many`/`for_shop`/`register_to_shop`/`unregister_from_shop`, `PartId::owner`, `Usage::delete_many`, `ServicePlan::delete_many`, `Service::delete_many`, `Activity::delete_many`, `Attachment::delete_by_parts`, `Notification::create`/`all_for_user`, `PartNote::all_for_part`, `UserId::set_onboarding_status`). The store traits, sqlx impls, and `MockStore` are unchanged — only callers reroute.
- The rule is machine-checked: `.githooks/check-store-access.py` (run by the `pre-commit` hook) asserts each `<entity>_<op>` store method is called only from its owning entity's file plus the exempted locations, and fails on anything else — the same grep-verifiable acceptance bar as ADR-0007.
- Three store methods that broke the `<entity>_<op>` convention — and so collided with entity-method names (`get_all` vs `Part::get_all`, `delete_all` vs `Usage::delete_all`) — are renamed to it: `ActivityStore::get_all` → `activity_get_all`, `PartStore::shop_get_parts` → `part_get_for_shop`, `UsageStore::delete_all` → `usage_delete_all`. With every store method now entity-prefixed, the check can match by name with no false positives.
- A future cross-entity operation is placed by the same rule: the callee exposes the operation it owns, the caller invokes the entity method, and the new method name is checked by the hook.
