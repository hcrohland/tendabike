# ADR-0016: A disabled provider is a graceful freeze, not an absence

A provider is **enabled** iff its primary credential (client id) is set; the enabled set is a runtime subset of the `Provider` enum, derived from configuration at startup and owned by the provider layer. We decided that turning a provider off *live* — one that already has users — is a **graceful freeze**, not an absence and not a data deletion: existing users keep reading their data, their syncs fail with a clear "provider disabled" error, they cannot re-authenticate (no OAuth without credentials), and re-adding the credential restores the provider and its syncs.

## Considered options

- **Absence** (a disabled provider is fully gone — no route, no data): clean in the fresh-setup case, but destructive in practice — it strands or loses existing users' data the moment a credential is removed, and makes turn-off irreversible.
- **Refuse to disable** (the backend refuses to start when a disabled provider has users): a safety valve, but it forbids a legitimate configuration (an instance that no longer uses a provider) and forces "delete the users first" — a data loss the operator did not ask for.
- **Graceful freeze** (chosen): no data loss, fully reversible, and the failure is loud and clear (syncs error with a stated reason) rather than silent. The cost is that a disabled provider is not *fully* inert — its routes exist and must degrade gracefully — but that cost is small and local.

## Consequences

- The per-provider credential fields in the config become optional: a missing client id means *disabled*, not a startup panic.
- The provider layer owns the enabled set (a runtime subset of the `Provider` enum); the web layer reads it through the provider facade, and the domain stays provider-ignorant (ADR-0013/0014/0015).
- A new unauthenticated endpoint exposes the enabled set (ordered wire values) so the frontend offers only the configured providers and shows a failure page when the set is empty.
- Every surface a disabled provider touches degrades gracefully: its login entry is hidden and direct hits return a clear "unavailable"; its webhook route is inert; its users' syncs fail with a clear error; their data remains readable.
