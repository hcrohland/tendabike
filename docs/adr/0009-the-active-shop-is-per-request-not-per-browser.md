# ADR-0009: The active shop is per-request, not per-browser

## Status

Accepted.

## Context

The acting session (`RequestSession`) bundles two things at two scopes: the caller's **identity** (`user_id`, `is_admin`) and its **context** — today, the active `shop`. The identity is genuinely per-browser: it lives in the tower-session cookie, shared by every tab in a browser. The active shop, however, is a per-tab concern: the client keeps it in per-tab module state (`getShop()`), and two tabs can legitimately act in different shops at once.

The active shop was being persisted into the per-browser cookie (`set_shop` on `GET /api/user?shop=`), so the server's acting session carried a *browser-wide* shop. That scope mismatch caused cross-tab interference: tab B entering a shop rewrote the shared cookie, and tab A's subsequent requests — including mutations that carry no shop — saw tab B's shop in their access checks and frame routing.

## Decision

**The active shop is per-request, not per-browser.** The acting session is split into an identity half (per-browser, from the cookie: `user_id`, `is_admin`) and a context half (per-request: the active shop). The shop stops being persisted in the cookie; `set_shop` is removed. The server is stateless with respect to the shop — it reads the shop from each request and stores nothing.

**Carriage.** Fetch requests (reads and mutations) carry the shop in a dedicated `X-Active-Shop` header. The SSE stream keeps `?shop=` as a query parameter, because the client uses the native `EventSource`, which cannot send custom headers. Both populate the same `Session::shop()`.

**Single source of truth on the client.** `getShop()` is the tab's shop. Both the stream URL (`?shop=`) and the fetch header (`X-Active-Shop`) are projections of it, and the header is injected centrally by the fetch layer (`myfetch`) rather than set per call — so the two can't diverge. The server is stateless and trusts the header; consistency is a client invariant, guaranteed by the shared source plus central injection.

**Frame routing for server-driven events.** The executor routes a write-frame to a shop stream using the acting session's shop. That works for client-driven operations (the request states the shop). A Strava event has no per-request shop, so once the cookie no longer carries one, **Strava-event frames go to the main stream only** — an open shop view catches up on its next refresh.

## Rejected alternatives

- **Keep the shop per-browser (cookie) and sync the tabs to it.** Rejected: it would force every tab in a browser to share one active shop, and require cross-tab synchronization (broadcast/storage events) the client doesn't have.
- **Persist the active shop on the user (per-user).** Rejected: it would be the same everywhere and survive device changes, which is not the intended scope — the shop is a per-view concern, not an account property.
- **Route Strava-event frames by content or by open subscriptions.** More correct (a frame would reach the shop stream of the shop the affected part belongs to), but it requires the executor to resolve a shop from the frame rather than the acting session. Deferred: main-stream-only is a small, self-healing live-update gap, and content-based routing is a larger change not worth it now.

## Consequences

- `RequestSession` keeps its shape (`id`, `is_admin`, `shop`) but `shop` is no longer written to the cookie; `set_shop` and the `GET /api/user?shop=` → `set_shop` wiring are removed. The `FromRequestParts` extractor reads the shop from the `X-Active-Shop` header (fetch) or `?shop=` (SSE stream) per request.
- `Session::shop()` and all its consumers — the `apiread.rs` user-read filter, `tb_exec` frame routing, and the `part.rs` access checks — are unchanged; they now read the per-request value.
- The client attaches its tab's shop to every fetch call via the `myfetch` wrapper and keeps the existing `?shop=` on the SSE stream.
- Strava-event frames no longer reach an open shop view live; the shop view reconciles on its next refresh.
