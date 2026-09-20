# AGENTS.md

**TendaBike** - A bike maintenance tracker that syncs with Strava: users track parts, schedule services, and log cycling activities imported from Strava.

## Working rules

- When in doubt about a change, ask the user before making it.

## Domain

- Each entity lives in both halves: `backend/domain/src/entities/<name>.rs` and `frontend/src/lib/<name>.ts`.

## Commit rules

- Flow, in order: finish the work → if on `main`, create a feature branch → run `cargo fmt` and `npm run format` from the project root → commit.
- The pre-commit hook enforces formatting, type checking, and linting; never bypass it with `--no-verify` or `-n` — make the checks pass instead.

## Agent skills

- Issue tracker: issues and specs live in GitHub Issues on `hcrohland/tendabike`; use the `gh` CLI, with one `feature:<slug>` label per feature. See [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).
- Triage labels: five-role vocabulary, each label string equal to its role name; see [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).
- Domain docs: single-context `CONTEXT.md` and `docs/adr/` at the repo root; see [`docs/agents/domain.md`](docs/agents/domain.md).
- Domain flow: when adding or modifying a mutating operation — a domain operation, an endpoint that serves it, or its client merge — read [`docs/agents/domain-flow.md`](docs/agents/domain-flow.md) first; the domain layer computes everything and responses cover the whole `Summary`.
- New entity: when adding a new entity, follow the wiring checklist in [`docs/agents/new-entity.md`](docs/agents/new-entity.md).
