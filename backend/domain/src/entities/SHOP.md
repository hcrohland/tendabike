# Shop & ShopSubscription Entities

## Overview

A `Shop` is a collection of parts owned by a shop owner. Other users **subscribe** to the shop; once their subscription is active, they can register their own parts (bikes and loose spares) to it, delegating maintenance to the shop owner.

```
Shop (owner)
  │
  ├── ShopSubscription (user, status: pending → active | rejected; cancellable)
  │        │
  │        └── active ──▶ user may register parts
  │
  └── parts.shop = shop.id   (registered parts: bikes + their cascaded subparts + spares)
```

The subscription is **user-level**, not per-part: one active subscription lets a user register any number of their own parts. Request/response messages are not stored on the subscription — they ride on the matching notifications (see [Side effects](#side-effects-notifications)).

## Data Model

### `Shop` (`domain/src/entities/shop.rs:48-65`)

| Field         | Type             | Description                                              |
| ------------- | ---------------- | -------------------------------------------------------- |
| `id`          | `ShopId`         | Primary key                                              |
| `owner`       | `UserId`         | The shop owner                                           |
| `name`        | `String`         | Display name                                             |
| `description` | `Option<String>` | Optional description                                     |
| `auto_approve`| `bool`           | If `true`, new subscriptions are activated immediately   |
| `created_at`  | `OffsetDateTime` | Creation timestamp                                       |

### `ShopSubscription` (`domain/src/entities/shop/subscription.rs:13-27`)

| Field        | Type               | Description                          |
| ------------ | ------------------ | ------------------------------------ |
| `id`         | `SubscriptionId`   | Primary key                          |
| `shop_id`    | `ShopId`           | The subscribed shop                  |
| `user_id`    | `UserId`           | The subscriber                       |
| `status`     | `SubscriptionStatus` | `pending`, `active`, `rejected`, `cancelled` |
| `created_at` / `updated_at` | `OffsetDateTime` | Timestamps (`updated_at` via DB trigger) |

`ShopSubscriptionWithDetails` (`subscription.rs:30-44`) is the API response shape: the same fields plus the embedded `shop: Shop`.

### The single-shop constraint

A part's shop membership is a **column, not a junction**: `parts.shop` is a nullable `ShopId` (migration `20260108103729_shop_to_parts`, which dropped the earlier `shop_parts` junction table). A part belongs to **at most one shop** at a time; registering it to a new shop moves it.

## Lifecycle Methods

### `ShopId` (`shop.rs`)

| Method | Description |
|--------|-------------|
| `get(id, user, store)` | Resolve a shop id, **ownership-only** access check |
| `get_for_read(id, user, store)` | Resolve a shop id for read access: owner **or** active subscriber |
| `check_owner(user, store)` | `Forbidden` unless the user owns the shop; returns the shop |
| `create(name, description, auto_approve, user, store)` | Create a shop |
| `update(name, description, auto_approve, user, store)` | Owner-only update |
| `delete(user, store)` | Owner-only; `Conflict` if the shop still has parts |
| `register_part(part_id, session, store)` | See [Registration](#registration--cascading) |
| `unregister_part(part_id, session, store)` | See [Registration](#registration--cascading) |
| `subscribe(message, user, store)` | Create a pending subscription (auto-approve path included); see [Side effects](#side-effects-notifications) |
| `approve_subscription(sub_id, message, user, store)` | Owner-only; pending → active |
| `reject_subscription(sub_id, message, user, store)` | Owner-only; pending → rejected |
| `cancel_subscription(sub_id, user, store)` | Subscriber-only; removes a pending/active/rejected subscription |
| `get_parts(user, store)` | **Owner-only** list of the shop's registered parts |
| `read(store)` | Public read |

### `SubscriptionId` (`subscription.rs`)

| Method | Description |
|--------|-------------|
| `create(shop_id, user, store)` | `Conflict` if a pending or active subscription already exists; auto-approve shops activate it immediately |
| `get(id, user, store)` | Access check: the subscriber, or the shop owner |
| `read(user, store)` | Read after access check |
| `approve(user, store)` | Owner-only; `Conflict` if not pending |
| `reject(user, store)` | Owner-only; `Conflict` if not pending |
| `cancel(user, store)` | Subscriber-only; `Conflict` if the subscriber still has parts in the shop |

`ShopSubscription` collection queries: `get_pending_for_shop` (owner-only, pending rows with shop details), `get_for_shop`, `get_for_user`, `with_shop_details`.

## Registration & cascading

`register_part` (`shop.rs:169-199`):

1. **Subscription gate** — `ShopSubscription::check` requires an *active* subscription row. **The shop owner is not exempt**: an owner must subscribe to their own shop before registering parts.
2. **Ownership gate** — the actor must own the part (`checkuser`).
3. **Unattached-only** — a part that is currently attached to a bike cannot be registered (`BadRequest`); register the bike and its attachments cascade.
4. **Cascade** — `parts_for_register` collects the part's current subparts (via the attachment rows) plus the part itself; `Part::register_to_shop` sets `parts.shop` on all of them. Registering a bike therefore registers its whole assembly.
5. Returns a `Summary` with the registered parts, the shop, and the notification.

`unregister_part` (`shop.rs:217-261`) mirrors this: the actor must own the part (the subscription gate from registration does **not** apply — a part owner can remove their own parts even after their subscription ends); the cascade clears `parts.shop` on the part and its subparts.

## Side effects: notifications

All side effects go through the [notification system](notification.rs) — the actor is never notified of their own action:

| Operation | Kind | Lifecycle | Recipient | Message |
|-----------|------|-----------|-----------|---------|
| `subscribe` (manual approval) | `sub_request` | persistent | shop owner | the request text |
| `subscribe` (auto-approve) | `sub_response` | persistent | requester | "Welcome to {shop}" |
| `approve_subscription` | `sub_response` | persistent | requester | the reply |
| `reject_subscription` | `sub_response` | persistent | requester | the reply |
| `cancel_subscription` | `sub_cancelled` | persistent | shop owner | — |
| `register_part` | `part_registered` | transient | shop owner | — (one per top-level part; the cascade does not fan out) |
| `unregister_part` | `part_unregistered` | transient | the *other* party: shop owner when the part owner acts, part owner when the shop owner acts; none when the actor is both | — |

## API surface (`axum/src/domain/shop.rs`)

```
GET    /api/shop                          - shops the user owns or has an active subscription to
POST   /api/shop                          - create a shop {name, description, auto_approve}
GET    /api/shop/search?q=                - search (below)
GET    /api/shop/{shop}                   - read (owner or active subscriber)
PUT    /api/shop/{shop}                   - update (owner)
DELETE /api/shop/{shop}                   - delete (owner; empty only)

GET    /api/shop/{shop}/parts             - registered parts (owner)
POST   /api/shop/{shop}/parts             - register {part_id} (active subscriber who owns the part)
DELETE /api/shop/{shop}/parts/{part}      - unregister (part owner)

GET    /api/shop/subscriptions            - the user's subscriptions (with shop details)
POST   /api/shop/subscriptions            - request {shop_id, message}
GET    /api/shop/subscriptions/{sub}      - read (subscriber or shop owner)
DELETE /api/shop/subscriptions/{sub}      - cancel (subscriber)
POST   /api/shop/subscriptions/{sub}/approve  - approve (owner) {message}
POST   /api/shop/subscriptions/{sub}/reject   - reject (owner) {message}
GET    /api/shop/{shop}/subscriptions     - pending requests for a shop (owner)
```

## Search

`shops_search` matches the query (case-insensitive `ILIKE`, `LIMIT 50`) against the shop name and the owner's first/last/full name, including cross-field combinations ("Bike Shop John", "John Bike Shop"):

```sql
WHERE g.name ILIKE $1
   OR COALESCE(u.firstname, '') ILIKE $1
   OR COALESCE(u.name, '') ILIKE $1
   OR CONCAT(COALESCE(u.firstname, ''), ' ', COALESCE(u.name, '')) ILIKE $1
   OR CONCAT(g.name, ' ', COALESCE(u.firstname, ''), ' ', COALESCE(u.name, '')) ILIKE $1
   OR CONCAT(COALESCE(u.firstname, ''), ' ', COALESCE(u.name, ''), ' ', g.name) ILIKE $1
```

## Permission model

| Action | Shop owner | Active subscriber | Pending/rejected/cancelled | Stranger |
|--------|:---:|:---:|:---:|:---:|
| Update / delete shop | ✓ | ✗ | ✗ | ✗ |
| Approve / reject subscriptions | ✓ | ✗ | ✗ | ✗ |
| Cancel subscription | ✗ (reject) | ✓ (own) | ✓ (own) | ✗ |
| Register a part | ✓¹ | ✓ (own parts) | ✗ | ✗ |
| Unregister a part | ✓ (any) | ✓ (own) | ✓ (own) | ✗ |
| View shop | ✓ | ✓ | ✗ | ✗ |
| View registered parts | ✓ (all) | ✗ | ✗ | ✗ |

¹ the owner needs an active subscription row of their own (see [Registration](#registration--cascading)).

## Frontend integration

- **Classes**: `frontend/src/lib/shop.svelte.ts` (`Shop`) and `frontend/src/lib/subscription.ts` (`ShopSubscription`), each with a `mapableState` collection (`shops`, `subscriptions`) updated by the Summary stream.
- **Shop mode**: `enterShop(shop)` / `exitShop()` swap the active shop in a module-level `$state` and re-fetch; the fetch layer injects the active shop via the `X-Active-Shop` header (ADR-0009), scoping reads to the shop.
- **Components** (`frontend/src/Shop/`): `Shops.svelte` (tabs: my shops / my subscriptions), `ShopList`/`ShopCard` (cards, part counts, owner info), `ShopSearch` (debounced `/search`), `Subscriptions`/`SubscriptionRow`/`ShopSubscriptions` (subscription tables, approve/reject/unsubscribe), `SubscriptionRequestModal` + `DeleteShop` + `ShopModal` (module-scoped modals opened via start functions, mounted in `Widgets/Actions.svelte`), `Register`/`ShopRegistration` (part registration), `ShopFrame`/`ShopMenu`/`ShopOwnerMenu` (shop-mode chrome).
- **Cross-component refresh**: `SubscriptionRequestModal` dispatches a `subscription-updated` window `CustomEvent`; `ShopCard` listens and reloads.

## Key files

| Layer | File |
|-------|------|
| Entity | `backend/domain/src/entities/shop.rs` (+ `shop/subscription.rs`) |
| Trait | `backend/domain/src/traits/shop.rs` |
| Store | `backend/sqlx/src/store/shop.rs` |
| API | `backend/axum/src/domain/shop.rs` |
| Frontend | `frontend/src/lib/shop.svelte.ts`, `frontend/src/lib/subscription.ts`, `frontend/src/Shop/` |
| Migrations | `20250131000000_add_garages` (shops table), `20250202000000_add_garage_subscriptions` (subscriptions), `20260108103729_shop_to_parts` (junction → `parts.shop` column), `20260112103119_shop_auto_approve`, `20261103000000_move_subscription_message_to_notifications` |
