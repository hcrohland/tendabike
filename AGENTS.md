# AGENTS.md

**TendaBike** - A bike maintenance tracker that syncs with Strava: users track parts, schedule services, and log cycling activities imported from Strava.

## Working rules

- Understand a type by its contract, not its methods. A type's contract is what it is and what it guarantees to callers — its module doc plus its public method signatures. Confirm it with 1–3 representative usages rather than reading every method or tracing every caller. Drop into the implementation only to implement or modify the type, debug, or when the contract is too thin to decide.
- When more than one option is defensible, ask the user before choosing — even when you're confident in your pick.
- Keep batch size one: one operation per call — one ticket per create, one file or function per edit — and confirm it landed before starting the next.
- Formatting is done by external formatters, never by hand: write the code, then run cargo fmt (Rust) or npm run format from the project root.

## Domain

- Each entity lives in both halves: `backend/domain/src/entities/<name>.rs` and `frontend/src/lib/<name>.ts`.

## Commit rules

- Flow, in order: finish the work → if on `main`, create a feature branch → commit → push → if the work is on an issue, comment on it with the commit SHA.
- Commit titles name the change; the issue number goes in a `Refs #N` line in the body, one per issue. `Refs` links the issue without closing it.
- Never bypass the pre-commit hook with `--no-verify` or `-n` — make the checks pass instead.

## Agent skills

- Issue tracker: when working with issues, specs, or PRs, read [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).
- Triage labels: five-role vocabulary, each label string equal to its role name; see [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).
- Domain docs: single-context `CONTEXT.md` and `docs/adr/` at the repo root; see [`docs/agents/domain.md`](docs/agents/domain.md).
- Domain flow: when adding or modifying a mutating operation — a domain operation, an endpoint that serves it, or its client merge — read [`docs/agents/domain-flow.md`](docs/agents/domain-flow.md) first; the domain layer computes everything and responses cover the whole `Summary`.
- New entity: when adding a new entity, follow the wiring checklist in [`docs/agents/new-entity.md`](docs/agents/new-entity.md).
