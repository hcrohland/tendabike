# AGENTS.md

## Workspace

`frontend/` is an npm workspace of the root `package.json`. Run all npm commands, including `npm install`, from the project root (`../`), where `node_modules` and `package-lock.json` live.

## Commands

- `npm run check` - Frontend type checking (compiles paraglide, then runs svelte-check)
- `npm run test` - Vitest. Never raw `npx vitest` from the repo root (no vite/Svelte config there → parse errors).
- Run a subset: `npm run test -- src/<file>.test.ts` from the repo root — the path is relative to `frontend/`, so it omits the `frontend/` prefix.
- **Prettier**: config lives in `frontend/.prettierrc` — run it via the root npm scripts (`npm run fmtcheck` / `npm run format`), never raw `npx prettier` from the repo root (no config there → defaults → false positives).

## Verification

- **Browser:** Playwright + Chromium is available system-wide. Use it for real-browser verification when a spec's acceptance criteria require it (e.g., confirming a page renders without console errors), or when a change alters what the user sees (color, contrast, layout) — a rendered result is not inferable from theme tokens, so render and look. The full local stack — database, backend, dev server, Playwright setup, teardown — is in [`docs/dev-stack.md`](docs/dev-stack.md).
- **jsdom's blind spot:** jsdom does not model flowbite's async popper wiring. For changes to `Menu`/Dropdown open-close behaviour — or any shared widget's interaction — a passing jsdom test is not sufficient: verify open, close, and keyboard reachability in a real browser before shipping.

## Architecture

- **UI**: Tailwind CSS v4 + flowbite-svelte (components) + flowbite-svelte-icons (icons). New UI comes from flowbite-svelte components — raw Svelte + Tailwind only where the library has no component.
- **Modals**: each modal owns its open state in module scope and exports a start function that opens it; [`src/Widgets/Actions.svelte`](src/Widgets/Actions.svelte) mounts every modal once, globally, so any page can trigger it. Callers import the start function from the modal module directly — there is no shared actions registry.
- **Routing**: SvelteKit file-based routing; routes in [`src/routes/`](src/routes/).
- **i18n**: `@inlang/paraglide-js` — edit source messages in `messages/{locale}.json`; `paraglide/` is compiled output.
- **State**: the Summary collections in `src/lib/*.ts` are Svelte 5 `$state` objects from `mapableState()` in [`src/lib/mapable.svelte.ts`](src/lib/mapable.svelte.ts) — records keyed by id with `setMap`/`updateMap`/`deleteItem` ops; enumerate them with `stateValues()`, never `Object.values` (the ops ride on the record).
- **State objects**: when adding or touching a `.svelte.ts` state module or its readers, read [`../docs/agents/state-object.md`](../docs/agents/state-object.md) first.
- **Door convention**: lib-layer functions read the module's state objects in their bodies; signatures carry only entity values and time — see the door rule in [`../docs/agents/state-object.md`](../docs/agents/state-object.md).
- **Entity classes**: `src/lib/*.ts` — async methods that call `myfetch()` and update collections via `updateSummary()` in [`src/lib/user.svelte.ts`](src/lib/user.svelte.ts).

## Code Style

- **Runes mode**: `<script lang="ts">` blocks with runes (`$state`, `$derived`, `$effect`, `$props()`); module scope via the `<script module>` attribute (never the deprecated `context="module"` form).
- **Widget wrappers**: use the shared `src/Widgets/` wrappers over raw flowbite components. The load-bearing three: `Card` (entity container), `Menu` (per-entity action dropdown of `DropdownItem`s), `Chip` (label + value metric).
- **Button color**: a button's background is the surface tone opposite its container — the same flip as `Chip`'s `light` prop. On a card (`bg-surface-2`): `color="light"` (white / gray-800). On the page base: `color="light" class="bg-surface-2!"` (gray-300 / gray-600). `color="light"` carries the right text color in both modes; a bare `<Button>` (default indigo) breaks the scheme.
- **Entity list pages**: one `Card` per entity in a card-list container, per-entity actions in a `Menu` in the card header. Reference: [`src/Activity/ActList.svelte`](src/Activity/ActList.svelte) + [`src/Activity/ActCard.svelte`](src/Activity/ActCard.svelte).
- **UI open/close state**: component-local — a lib state module never carries it; a child that must close the UI takes a `close` callback prop.

## Key Gotchas

- `myfetch()` returns `null` for HTTP 204 — callers must handle it.
- Collection updates must use `updateMap()` / `setMap()` / `deleteItem()` on the entity's exported collection (e.g. `parts.updateMap([data])`).
- Paraglide messages are imported from `paraglide/messages` (relative depth varies by component); translation keys follow the `m.action_name()` pattern.
- `src/test/setup.ts` restores all mocks in `afterEach` — `vi.stubGlobal` stubs created in `beforeAll` are gone after the first test; create stubs in `beforeEach`.
- Svelte 5 `bind:value` on a raw `<input type="text">` listens to `input`, not `change` — drive it with `fireEvent.input`. `DateTime.test.ts`'s `fireEvent.change` works only because it targets a flowbite `Input`.
- **flowbite-svelte Popper/Dropdown in tests**: the trigger fires on `mousedown` (not `click`); open/close is `requestAnimationFrame`-debounced, so fake timers must also fake `rAF` + `performance`; Svelte 5 transitions complete via the Web Animations API (`element.animate` → `onfinish`), which is why `setup.ts` stubs `animate`/`showPopover`. Drive these tests with `vi.advanceTimersByTimeAsync` + `flushSync` in an `async` body.
