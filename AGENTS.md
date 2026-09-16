# AGENTS.md

**TendaBike** - A bike maintenance tracker that syncs with Strava: users track parts, schedule services, and log cycling activities imported from Strava.

## Pointers

- Svelte 5 + TypeScript frontend: read [`frontend/AGENTS.md`](frontend/AGENTS.md) before editing anything under `frontend/` (commands, state pattern, myfetch, gotchas).
- Rust workspace backend: read [`backend/AGENTS.md`](backend/AGENTS.md) before editing anything under `backend/` (commands, crates, layering, conventions, gotchas, test guides).
- Backend tests: in-memory, no database needed — run `SQLX_OFFLINE=true cargo test`; suites documented in [`backend/AGENTS.md`](backend/AGENTS.md).

## Commands (run from project root)

- `npm run dev` - Frontend dev server; Vite proxies `/api` and `/strava` to the backend on `:8000`
- `npm run check` - Frontend type checking (svelte-check; the CI variant `check:ci` also compiles paraglide first)
- `docker build -t tendabike .` - Full image (Rust + Node multi-stage)

## Domain

- Activities sync in from Strava; usage accumulates on parts; service plans schedule maintenance from usage (time/distance/effort); parts can be delegated to shops.
- Each entity lives in both halves: `backend/domain/src/entities/<name>.rs` and `frontend/src/lib/<name>.ts`.
- API routes: `/api/{user,types,shop,part,service,plan,activ}` and `/strava/*` (OAuth, webhook); part attachments nest under `/part`.

## Commit rules

- Run `cargo fmt` and `npm run format` from the project root before committing; the pre-commit hook enforces formatting, type checking, and linting.
- Never bypass the hook with `--no-verify` or `-n`; make the checks pass instead.
- The user reviews all changes before they are committed.

## Agent skills

- Issue tracker: issues and specs are local markdown under `.scratch/<feature>/`; see [`docs/agents/issue-tracker.md`](docs/agents/issue-tracker.md).
- Triage labels: five-role vocabulary, each label string equal to its role name; see [`docs/agents/triage-labels.md`](docs/agents/triage-labels.md).
- Domain docs: single-context `CONTEXT.md` and `docs/adr/` at the repo root; see [`docs/agents/domain.md`](docs/agents/domain.md).
