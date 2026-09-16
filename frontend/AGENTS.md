# AGENTS.md

## Commands (run from `frontend/`)

- `npm run dev` - Dev server (Vite, HMR)
- `npm run build` - Production build to `dist/`
- `npm run check` - svelte-check type checking
- `npm run check:ci` - Paraglide compile + svelte-check; CI runs this
- `npm run test` - Vitest suite (jsdom); compiles paraglide first
- `npm run format` / `npm run fmtcheck` - Prettier; CI runs `prettier --check`

**Tests**: live in `src/**/*.test.ts` (Vitest + `@testing-library/svelte`; coverage via `npm run test:coverage`). [`src/test/setup.ts`](src/test/setup.ts) pins the Paraglide locale to `en` and restores all mocks after each test.

## Architecture

- **UI**: Tailwind CSS v4 + Flowbite / flowbite-svelte components.
- **Routing**: `svelte-spa-router`; route definitions in [`src/App.svelte`](src/App.svelte).
- **i18n**: `@inlang/paraglide-js` — edit source messages in `messages/{locale}.json`; `paraglide/` is compiled output.
- **State**: custom `mapable()` pattern in [`src/lib/mapable.ts`](src/lib/mapable.ts) wrapping Svelte writable stores.
- **API**: centralized [`myfetch()`](src/lib/store.ts) wrapper in `src/lib/store.ts`, with `checkStatus()` error handling (401 redirects, message display).
- **Entity classes**: `src/lib/*.ts` — async methods that call `myfetch()` and update stores via `updateSummary()` in [`src/lib/user.ts`](src/lib/user.ts).

## Code Style

- **Runes mode**: `<script lang="ts">` blocks with runes (`$state`, `$derived`, `$effect`, `$props()`). Exception: [`src/Widgets/Actions.svelte`](src/Widgets/Actions.svelte) is legacy syntax (`export let`, `$:` statements, `context="module"`) — leave it as-is.
- **Two `<script>` blocks**: `module` block for top-level awaits (e.g. `await getTypes()`), regular block for component logic.
- **Entity classes**: `constructor(data: any)` mapping API response fields; Date fields wrapped in `new Date()`.
- **License header**: required at the top of new `.svelte` files (copy from [`src/App.svelte`](src/App.svelte)).

## Key Gotchas

- `myfetch()` returns `null` for HTTP 204 — callers must handle it.
- Store updates must use `updateMap()` / `setMap()` / `deleteItem()` on the entity's exported store variable (e.g. `parts.updateMap([data])`).
- Paraglide messages are imported from `paraglide/messages` (relative depth varies by component); translation keys follow the `m.action_name()` pattern.
- `svelte-spa-router` params arrive via `$props()`, typed like `{ id: number }`.
- `src/test/setup.ts` restores all mocks in `afterEach` — `vi.stubGlobal` stubs created in `beforeAll` are gone after the first test; create stubs in `beforeEach`.
