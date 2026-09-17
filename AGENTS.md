# AGENTS.md

**TendaBike** - A bike maintenance tracker that syncs with Strava: users track parts, schedule services, and log cycling activities imported from Strava.

## Domain

- Each entity lives in both halves: `backend/domain/src/entities/<name>.rs` and `frontend/src/lib/<name>.ts`.

## Commit rules

- Run `cargo fmt` and `npm run format` from the project root before committing; the pre-commit hook enforces formatting, type checking, and linting.
- Never bypass the hook with `--no-verify` or `-n`; make the checks pass instead.
- The user reviews all changes before they are committed.

## Agent skills

- Issue tracker: issues and specs are local markdown under `.scratch/<feature>/`; see [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).
- Triage labels: five-role vocabulary, each label string equal to its role name; see [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).
- Domain docs: single-context `CONTEXT.md` and `docs/adr/` at the repo root; see [`docs/agents/domain.md`](docs/agents/domain.md).
