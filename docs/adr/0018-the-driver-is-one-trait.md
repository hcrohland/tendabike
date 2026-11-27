# ADR-0018: The driver is one trait

## Status

Accepted. The record of the provider driver's shape: one driver trait, and the marker's `Driver` as its single home — the lane drives the marker's driver. Joins the same seam family as ADR-0013 (concepts follow the dependency graph), ADR-0014 (the provider service is one interface, one impl), and ADR-0016 (a disabled provider is a graceful freeze).

## Context

The provider layer's driver interface was split across two traits with identical implementors: the provider driver trait (the provider-specific core plus the shared user-lifecycle skeleton) and a narrow token-refresh trait (the refresh exchange and the refresh-token accessor). The split had outlived its purpose: the kit's token machinery bounded on the narrow trait, and one test stub (the queue test's driver) implemented only it to stay small.

The cost: the standard oauth2 refresh-token exchange — the same call in every oauth2 provider — was re-implemented verbatim in each provider driver, and durable-token providers and test stubs were forced to write no-op `refresh` overrides that existed only because the method was required. A provider author integrating a new provider implemented two traits; the no-refresh-token failure rule lived in the token state's refresh flow, not in the exchange; and the driver's client — the exchange's input — was reached by each driver impl on its own, with no shared name for the client shape the exchange needs.

## Decision

**The driver is one trait.** The narrow token-refresh trait is deleted; its surface moves onto the driver trait, beside the other row/token hooks: the token-response and client associated types, the shared-client function (a static reference), the `refresh` method (a default — the kit's refresh exchange over the provider's shared client, taking an optional refresh token), and the refresh-token accessor (a default: absent). A provider supplies only what is genuinely provider-specific: Strava supplies the token response, the client, the client function, and the refresh-token accessor; intervals.icu supplies the token response, the client, and the client function only. No provider overrides `refresh`.

**The refresh exchange is a kit free function.** The exchange — the standard oauth2 refresh-token call — is written once in the token module, generic over the provider client's token response type: an absent refresh token is the `NotAuth` (the single home of the no-refresh-token rule, the existing message preserved verbatim), an exchange failure maps to `NotAuth`, and the success returns the refreshed-token value (the new access token, the optional expiry, the optional rotated refresh token — the persist signal). The token state's refresh-if-expired drops its own no-token check and passes the option through.

**The client shape is named once.** A helper trait in the shared client module names the full oauth2 client shape — the common provider client's exact endpoint-marker set (the auth and token URLs set, the device-auth and introspection URLs unset, the revocation URL free) — blanket-implemented for every provider client; the driver trait's `Client` associated type is bounded by it. The trait exists because the common provider client is a type alias, which cannot appear in a trait bound. The trait carries the one method the exchange drives (the refresh-token request), so the exchange is generic over the bound, not over the concrete client type.

**The marker's `Driver` is the driver's single home.** The marker's `Driver` associated type is bounded by the full driver trait (previously the narrow token trait). The lane drives the marker's driver, and the lane's own driver type parameter is dropped (the per-provider lane, the unified lane, and the production alias) — so the type system enforces the driver a lane drives, rather than the production wiring matching the pair by convention.

**The durable-token model is documented at the impl.** A durable-token provider (intervals.icu) omits the `refresh` override entirely: its token path reads the durable token directly from the user row and never reaches the token cache's refresh flow, so its inherited exchange is never called. "This provider's tokens never refresh" is readable where the driver is read.

## Rejected alternatives

- **Narrow-trait segregation** (keep the token-refresh trait as the kit's bound and the test stub's surface): its only remaining consumers were the kit's `Driver` bound and one small test stub, and the cost — two traits per provider, the exchange re-implemented per oauth2 provider, no-op `refresh` overrides on durable-token providers and stubs, and the no-token rule living outside the exchange — outweighed the benefit. The accepted cost is the queue test stub's surface growth: it now implements the full driver trait, with inert stubs for the provider-specific core (never called from the queue ops) plus a dummy client (locally-pointing auth/token URLs).
- **Merging the marker into the driver** (making the marker the driver): the marker is deliberately type-only — the store, the token state, the token cache, and the queue read are all generic over it without naming the driver. The marker declares the wire shapes; the driver is the behavior. The two stay separate, and the marker's `Driver` is the typed link between them.

## Consequences

- A new provider implements one trait: the driver. The oauth2 boilerplate (the refresh exchange) is inherited as a default; the provider supplies the client and, where applicable, the refresh-token accessor.
- The no-refresh-token failure surfaces as a `NotAuth` from a single location in the exchange, regardless of which provider's driver is driven.
- The marker's `Driver` is the single home of the driver type: the lane drives `P::Driver`, and a mismatch between the lane's driver and the marker's driver no longer compiles.
- The refreshed-token value is publicly reachable through the driver module (it is named in the public trait signature).
- The change is behavior-preserving: the token lifecycle (cache, refresh-if-expired, rotated-token persistence), the lane's probe/process contract, and the executor and web-layer seams are all unchanged — the existing suites stay green at the existing seams, plus one new pure test for the one new branch (the absent-refresh-token `NotAuth`).
- The web layer's oauth modules import the driver trait; the surface they use (the authorize URL, the login) is unchanged.
