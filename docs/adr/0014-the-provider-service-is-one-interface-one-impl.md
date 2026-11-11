# ADR-0014: The provider service is one interface, one impl

## Status

Accepted. The record of the provider axis: the interface a presentation layer drives the provider's direct ops through, and the shape of its seam. Sibling record to ADR-0012 (the executor's write path); together the two cover both paths into the domain.

## Context

A presentation layer drives two kinds of ops. The message-lane ops (reads and writes that ride the per-user executor) are ADR-0012's territory. The **direct ops** — the provider's user lifecycle (login, webhook intake, the admin sync, deauthorize, delete) and the redirect URLs — are ops the `Registry` runs in its own transaction, never riding the executor's message lane. Their surface was settled in spec #558 §5.2, which predates the second provider.

With one provider the surface was a single trait — `ProviderService` — that was in fact Strava's. When intervals.icu landed, a parallel `IcuService` was added and five of Strava's eleven ops re-declared with an `icu_` prefix: three of them signature-identical (`sync`, `deauthorize`, `delete_user`), two differing in shape (the login's athlete and token types, the webhook's event type). The web state grew a third handle encoding "two named providers": a third provider would have been a new trait, a new handle, and a reshaped `AppState`. Two of the eleven ops on the Strava trait (`rescan`, `subscribe_shop`) were not provider concepts at all — a pure domain op and a shop stream, called from provider-neutral routes through a handle named `provider`.

## Decision

**One interface, one impl.** The provider's direct ops are one trait — `ProviderService` in `tb_exec` — implemented once, by the `Registry`. Eight ops, all provider-agnostic at the interface: `login`, `accept_event`, `sync`, `deauthorize`, `delete_user`, `activity_url`, `gear_url`, `user_url`. The provider's identity is a typed `Provider` enum in the provider layer's root — the typed form of the domain concept (the glossary: a user has exactly one provider, chosen at signup and immutable) — and every op dispatches through it: `login` and `accept_event` take it as a parameter (the route knows it from the URL), `sync`/`deauthorize`/`delete_user`/`user_url` parse `user.provider` in the op's own transaction, `gear_url` parses it from the part's owner, and `activity_url` parses the activity id's prefix. One dispatch mechanism for the whole interface; the same identity rule the executor's lane already uses.

**Opaque input at the seam.** The presentation layer extracts the HTTP values (the query params, the raw body) and forwards them; the provider layer does everything provider-specific that follows — the scope check, the token exchange, the wire-shape parsing, the webhook secret verification, the event filtering, the queue insert. The seam takes opaque input only where provider behaviour follows. A protocol handshake with no provider behaviour behind it stays in the web shell: Strava's subscription validation (the hub challenge) is two constant comparisons against a value axum already deserializes, and it does not cross the seam.

**The identity record.** The `Provider` enum is the single home for the provider's string encodings: its value in the `users.provider` column, the activity id's prefix, and the web route prefix. The axum nest sites take the prefix from the enum; the per-provider route tables stay per-provider (they are genuinely different). A third provider is a variant carrying all three encodings at once.

**The placement rule.** A single-target provider op goes on `ProviderService`, keyed by the identity its input carries: a parameter when no user exists yet (`login`, `accept_event`), the user row or the id's prefix when one does (`sync`, `deauthorize`, `delete_user`, `gear_url`, `user_url`, `activity_url`). A batched admin op goes on `ServiceApi` beside `all_stats`: `trigger_processing` is the batched form of `sync`, each user queued on their own provider's queue. A non-provider op goes on `ServiceApi`. No provider has an op the other lacks, so no extras subtrait is needed; if a future provider gains an exclusive op, a per-provider subtrait is introduced then, not before.

**The neutral ops go home.** `rescan` and `subscribe_shop` move to `ServiceApi`: they are not provider concepts, and provider-neutral routes stop reaching through a provider handle to call them.

## Rejected alternatives

*The seam's shape*

- **Per-provider twin traits** (today's `ProviderService` + `IcuService`): the shared half of the surface is spelled out twice, the generic-sounding name is a lie, and a third provider is a new trait plus a new handle.
- **A raw-string key** (a `&str` provider discriminator instead of the enum): the typos and the drift are uncheckable; the enum is the typed form of a concept the glossary already names.
- **Type-erasure enums for the differing wire types** (an athlete enum, an event enum, so every op shares one signature with no key): the erasure *is* the raw string the presentation layer already holds — the opaque input. Typed erasure built for a hypothetical third provider is machinery that buys nothing the string does not.
- **Adapter wrappers** (a per-provider newtype over the shared `Registry`, each implementing `ProviderService`, so the identity rides the handle): the two wrappers are projections of one impl, not independent implementations — a formal second adapter. The cost was a guaranteed per-provider handle and a provider identity hidden in the handle choice, in exchange for a typed parameter that states it honestly.

*The dispatch*

- **Handle-keyed dispatch** (the /strava route can only sync a Strava user; today's behaviour): the identity rule lives in two places (the lane internally, the routes by convention), and a per-provider admin route acting on a user of the other provider fails on a missing lookup instead of a stated rule.

*The handshake*

- **Moving the subscription validation into the provider layer**: the interface (three strings in, one string out) would be as complex as the implementation (two constant comparisons). Delete it and the complexity does not concentrate anywhere — it does not earn its keep. The credential it guards is a value the web shell already holds, beside its other protocol secrets.

## Consequences

- The web state is two handles: `api: Arc<dyn ServiceApi>` (eight ops — the original five plus `rescan`, `subscribe_shop`, and `trigger_processing`) and `provider: Arc<dyn ProviderService>`. The per-provider handles vanish; the `icu_` prefixes drop with the twin trait.
- The `Provider` enum lives in the provider layer's root: the domain stays provider-ignorant (it stores the string; the provider layer owns the typing — the same dependent-owned abstraction as the id prefixes under ADR-0013).
- The web modules shrink to the protocol: the extraction, the HMAC CSRF state, the callback shapes, the tower-session write, the status mapping, the hub handshake. `RequestSession` moves to a neutral home — both oauth flows share it, and it is not Strava's.
- A third provider costs: an enum variant (with its three encodings), a feature module in the provider layer, and match arms in the `Registry` impl. No `AppState` reshaping — the handles are the axis, not the providers.
- The seam's reality rests on the same footing as `ServiceApi` (ADR-0012): one production impl, the test mock, the named CLI driver.
- A follow-up tightening is noted, not done here: the executor's lane dispatches on the raw string (`provider == "icu"`); the enum gives it a typed switch.
