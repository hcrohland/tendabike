# ADR-0008: Guards live where the invariant lives

## Status

Accepted.

## Context

The `ApiWrite` dispatch (`exec` in `tb_domain`) applies one mutating route per arm. Several arms carried guards — an id-mismatch check, an ownership check, a filename/name default — that belonged to neither the domain op they called nor the presentation: they sat in the dispatch arm, and the reason for each lived only in git history. Two styles coexisted in the match ("call the op" vs "inline the guard + op"), and one guard (the activity id-mismatch) was duplicated across the axum handler and the dispatch arm. A future reader could not tell, from the code alone, which guards were intentional domain invariants and which were transport artifacts.

## Decision

**A guard lives at the layer that owns the invariant it enforces.** Presentation-boundary invariants stay in the presentation; domain invariants move into the domain op.

For the dispatch guards that are domain invariants, the guard moves into the op it protects and the dispatch arm becomes a flat "call the op + wrap":

- The **service-ownership** check moves into `Service::create`, which gains the session and the shop-store bound and runs the part's ownership check internally — aligning it with its siblings `redo` and `update`, which already do this (and fixing the one service op that omitted it).
- The **part-note file defaulting** (blank `name`/`filename` fallback) moves into `note_create_file`, which accepts the raw options and resolves them internally — aligning it with `update_file`, which already does this.

The **activity id-mismatch** check (path id must equal the body's id) is a presentation-boundary invariant, so it is de-duplicated to the axum handler and removed from the dispatch: the dispatch variant drops its path-id field, and the domain op is left untouched, because a domain op has no path-id concept and expressing the check there would leak an HTTP notion into the domain.

The onboarding status guards are deliberately **not** moved: they stay inline in the dispatch. They have no existing domain op to land in, and the Strava sync event they guard must stay loop-side (the domain cannot depend on the Strava driver), so extracting them would be a net-new op for a small win.

## Rejected alternatives

- **Move the id-mismatch into `Activity::update` by adding a path-id parameter.** Rejected: the domain op would then take a presentation-level value (the path id) it has no business knowing, breaking the method-on-the-entity shape. The invariant (path id == body id) is a property of the request, so it belongs in the presentation.
- **Keep the id-mismatch in both the handler and the dispatch (defense in depth).** Rejected: it leaves the two styles coexisting in the match and duplicates the same message string in two layers; the dispatch is the one place this refactor wants to read flat.
- **Move all four guard kinds, including the onboarding status guards.** Rejected for scope: the status guards have no existing op home and the Strava sync event must stay loop-side, making this the largest lift for the smallest win.

## Consequences

- The `exec` match reads as a single style (every arm calls an op and wraps the result); the guards that remain are the ones the dispatch genuinely owns.
- `Service::create`'s signature changes (gains a session and a shop-store bound); its callers — including `redo`, which drops its now-redundant explicit ownership check — and the test suites are updated.
- `note_create_file`'s signature changes (the name becomes an option it resolves internally); its single dispatch caller and test are updated.
- The activity dispatch variant drops its path-id field; the axum handler keeps the check.
- A future guard is placed by the same rule: ask which layer owns the invariant, and put the guard there.
