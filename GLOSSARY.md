# Tendabike

A bike maintenance tracker that syncs with Strava: users track parts, schedule services, and log cycling activities.

## Language

### Sync

**Summary**:
The payload an operation or sync endpoint returns, carrying the entities it produced or touched, keyed by id. In a full read it carries all of the user's entities; in responses to mutations or event draining it carries only the changed entities. A deleted entity appears as a tombstone.
_Avoid_: snapshot, diff (each names only one of the two readings)

**tombstone**:
A null entry in a Summary for an entity that no longer exists: the id tells the client which row to remove; the entry carries no entity data.

**live**:
An entry in a Summary that carries its entity's data; the opposite of a tombstone.

**wake**:
The in-memory signal the webhook fires after queueing a Strava event, making a running executor check the queue at once, even mid-idle-wait. It never spawns an executor — a queued event waits for the next spawn (an SSE connect or API write).
_Avoid_: ping, notification, kick

### Notifications

**notification**:
A message addressed to a single recipient, created as a side effect of a domain operation — never directly by the user. It has a kind and a lifecycle.
_Avoid_: wake (the executor's signal), alert (a plan's due-ness severity), message

**ephemeral**:
A notification's lifecycle: it appears as a toast and is gone when dismissed — never in the history, never in the unread count.

**transient**:
A notification's lifecycle: it appears in the history while unread and is deleted when read.

**persistent**:
A notification's lifecycle: it remains in the history after being read; its read state is recorded.

### Bike model

**type**:
The kind of thing a part is (e.g. tire, chain, wheel). Every type points by its main reference at the category it belongs to.

**category**:
A type which cannot be attached to another type — a top type such as bike, ski, or shoe.
_Avoid_: main (that is the name of the reference a type points at its category by)

**gear**:
A part whose type is a category — a bike, a ski.

**assembly**:
A part and the parts attached to it at a time, including parts attached to those.

**hook**:
A mount position that a type can attach to, expressed as the type of the part it attaches onto (a tire hooks onto a wheel). The word is overloaded in the codebase with the Strava event-drain endpoint; in domain discussion it means the mount position.

**attachment**:
A part being mounted on another part at a hook, over a span of time. It is the part's place in the assembly: directly on the part it hooks onto, and thereby on the gear.
_Avoid_: file

### Session

**acting session**:
The caller's identity and context as a domain operation runs. Carried with each request; immutable for the operation's duration — the domain reads it, never mutates it. The **identity** (who the caller is) is stable for the browser; the **context** is per-request. Today the context is the active `shop` — the shop the caller is acting in, stated per request rather than held as a persisted browser-wide state, so a caller's tabs can each act in their own shop. The context is open to carrying more.
_Avoid_: session (the connection also has a cookie session and the loop a Strava session), user id

### Providers

**provider**:
The external system a user's activities come from, and the one the user authenticates through. A user has exactly one, chosen at signup and immutable. Today: Strava; intervals.icu is the second.
_Avoid_: source (a part's external id), lane (the dispatch abstraction)

**provider layer**:
The `tb_provider` crate: the provider-agnostic seams (the `EntityFacade` entity-op door, the `ProviderLane` lane seam, the rate-limit kernel) plus the provider integrations, each a feature module. Today the Strava driver is its `strava` module; a future provider adds its own feature module.
_Avoid_: provider (the external system a user syncs from — not the crate)

**provider kit**:
The shared scaffolding in `tb_provider` crate-root modules that every provider module depends on: the failure policy, the lane marker, the rate-limit shim, the HTTP classification, the client builder, the activity-type table, and the test-support machinery. A provider module carries only what is genuinely its own — the wire shapes, the token model, the queue semantics.
_Avoid_: provider layer (the whole crate, not just the shared scaffolding)

**provider activity id**:
The provider layer's typed activity id: the cross-provider form the opaque `ActivityId` converts to and from, and the single home for the `strava:` / `icu:` prefix knowledge. The domain's `ActivityId` stays an opaque string — the prefix is a provider concern (ADR-0013).
_Avoid_: activity id (the domain's opaque `ActivityId`)
