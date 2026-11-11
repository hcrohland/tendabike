# AGENTS.md

**TendaBike** - A bike maintenance tracker that syncs with Strava: users track parts, schedule services, and log cycling activities imported from Strava.

## Working rules

- Work from the interface, not through it. A type's public interface — its module doc and method signatures — is the complete specification. Confirm it with 1–3 representative usages. When the interface is unchanged, callers are out of scope; when calling a function, its internal dependencies are out of scope. Drop into implementation only to modify the type, debug, or when the contract is too thin to decide.
- The boundary holds at the package level too: a package's public API is its complete interface, even with a single consumer or implementor. Confirm the API; the implementation behind it is out of scope.
- Keep batch size one: draft a unit, write it, confirm it landed, then draft the next — one ticket per create, one file or function per edit. Each unit's content is written after the previous one lands, not drafted in a batch up front.
- Formatting is done by external formatters, never by hand: write the code, then run cargo fmt (Rust) or npm run format from the project root.

## Backend layout

A Cargo package's directory is its name without the `tb_` prefix: `tb_provider` → `backend/provider/`.

## Domain

- Each entity lives in both halves: `backend/domain/src/entities/<name>.rs` and `frontend/src/lib/<name>.ts`.

## Database

SQLx compiles queries against the committed `backend/sqlx/.sqlx/` offline cache. After adding or changing a query, run `cargo sqlx prepare` to refresh it; apply schema changes with `cargo sqlx migrate run`. A migration is frozen once applied — change its behaviour with a new migration, not by editing the applied one.

## Commit rules

- Flow, in order: finish the work → if on `main`, create a feature branch → commit optimistically (run `git commit` directly; fix and re-commit on hook failure) → push → if the work is on an issue, comment on it with the commit SHA.
- Commit titles name the change; the issue number goes in a `Closes #N` line in the body, one per issue. `Closes #N` auto-closes the issue on merge, so use it in the commit body — do not call the issue API to change the issue's state.
- Never bypass the pre-commit hook with `--no-verify` or `-n` — make the checks pass instead.

## Agent skills

- Issue tracker: when working with issues, specs, or PRs, read [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).
- Triage labels: five-role vocabulary, each label string equal to its role name; see [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).
- Domain docs: single-context `GLOSSARY.md` and `docs/adr/` at the repo root; see [`docs/agents/domain.md`](docs/agents/domain.md).
- Domain flow: when adding or modifying a mutating operation — a domain operation, an endpoint that serves it, or its client merge — read [`docs/agents/domain-flow.md`](docs/agents/domain-flow.md) first; the domain layer computes everything and responses cover the whole `Summary`.
- New entity: when adding a new entity, follow the wiring checklist in [`docs/agents/new-entity.md`](docs/agents/new-entity.md).
