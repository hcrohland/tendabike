# AGENTS.md

## Commands (run from project root)

- `npm run check` - Frontend type checking (svelte-check; the CI variant `check:ci` also compiles paraglide first)

## Architecture

- **UI**: Tailwind CSS v4 + Flowbite / flowbite-svelte components.
- **Routing**: `svelte-spa-router`; route definitions in [`src/App.svelte`](src/App.svelte).
- **i18n**: `@inlang/paraglide-js` — edit source messages in `messages/{locale}.json`; `paraglide/` is compiled output.
- **State**: the Summary collections in `src/lib/*.ts` are Svelte 5 `$state` objects from `mapableState()` in [`src/lib/mapable.svelte.ts`](src/lib/mapable.svelte.ts) — records keyed by id with `setMap`/`updateMap`/`deleteItem` ops; enumerate them with `stateValues()`, never `Object.values` (the ops ride on the record).
- **State objects**: when converting a `svelte/store` to Svelte 5 state or touching a `.svelte.ts` state module, read [`../docs/agents/state-object.md`](../docs/agents/state-object.md) first.
- **Entity classes**: `src/lib/*.ts` — async methods that call `myfetch()` and update collections via `updateSummary()` in [`src/lib/user.ts`](src/lib/user.ts).

## Code Style

- **Runes mode**: `<script lang="ts">` blocks with runes (`$state`, `$derived`, `$effect`, `$props()`). Exception: [`src/Widgets/Actions.svelte`](src/Widgets/Actions.svelte) is legacy syntax (`export let`, `$:` statements, `context="module"`) — leave it as-is.

## Key Gotchas

- `myfetch()` returns `null` for HTTP 204 — callers must handle it.
- Collection updates must use `updateMap()` / `setMap()` / `deleteItem()` on the entity's exported collection (e.g. `parts.updateMap([data])`).
- Paraglide messages are imported from `paraglide/messages` (relative depth varies by component); translation keys follow the `m.action_name()` pattern.
- `src/test/setup.ts` restores all mocks in `afterEach` — `vi.stubGlobal` stubs created in `beforeAll` are gone after the first test; create stubs in `beforeEach`.
