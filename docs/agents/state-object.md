# State Objects

How the Tendabike client holds shared state: Svelte 5 state objects in `.svelte.ts` modules. Read this when adding or touching a `.svelte.ts` state module or its readers.

## The four shapes

Pick by what the value is:

1. **Collection keyed by id** — the `mapableState` factory (`frontend/src/lib/mapable.svelte.ts`): a `$state` record of entities by id with `setMap`/`updateMap`/`deleteItem` attached. Reference: `part.ts`.
2. **Object, always defined** — `export const x = $state({...})` in `x.svelte.ts`; writes are property mutation. Reference: `message.svelte.ts`; `store.ts` re-exports the state.
3. **Object, nullable or replaced wholesale** — a module-private `let x = $state(...)` in `x.svelte.ts`, exposed through `getX()`/`setX()`; the entity module re-exports the accessors. Reference: `user.svelte.ts`; `user.ts` re-exports the accessors.
4. **Reference collection, loaded once** — a plain module-level `Map` in `x.ts`, written only by the loader and never again. Not reactive, because nothing can change it mid-session (ADR-0004). Reference: `types.ts`.

**Why shape 3 is accessors, not an exported state.** Svelte refuses to compile a `.svelte.ts` module that exports state it reassigns (`state_invalid_export`), and it equally forbids exporting a `$derived` (`derived_invalid_export`). A nullable value must be reassigned to cross `undefined` — a property mutation cannot conjure an object out of `undefined` — and a stable holder object is always truthy, which breaks `{#if x}` gates. The value therefore stays module-private, and the sanctioned fix (Svelte's own error message) is a function returning it.

## Mechanics

- **The `.svelte.ts` suffix is the contract**: only `.svelte.ts`/`.svelte.js` files are compiled by Svelte, so `$state` only works there. Import without the `.ts` stub: `import { ... } from "./user.svelte"` resolves to `user.svelte.ts`.
- **The entity module re-exports** (`user.ts`: `export { getUser, setUser };`) so readers keep one import source per entity.
- **Reader sites** — read the value where it is exposed, keeping each site's null style (`?.` / `!` / gate) on every property read (no narrowing across calls in the accessor shape):
  - state object: `x`, `x.prop`.
  - accessor shape: `getUser()`, `getUser()?.id`, `getUser()!.id`.
  - writes: `setUser(v)`; property mutation for shape 2.
- **Reactivity** — a state read inside a plain function registers dependencies at the enclosing reactive call site (the "door" shape, verified in #322/#346). Shape 2 tracks property mutation at any depth through its stable proxy. Shape 3 does the same: a re-assigned value is state-proxied like the initial one (not a plain object), so the replacement and any property mutation of it are both tracked; the current shape-3 state objects only write wholesale replacement, so the replacement leg is all the app exercises.
- **Door signatures** — lib-layer functions read the module's state objects in their bodies; signatures carry only entity values and time.
- **Enumerate collections with `stateValues(c)`.** The write operations attach to the record, so `Object.keys`/`Object.values`/`filterValues` see them; value-iteration over a state collection surfaces the operations as entries.

## Tests

- **Read idiom** — a state object is a plain value, not a store: assert on the direct read — `x["id"]`, `x.prop`, or `getX()`, never `get(x)`.
- **Component reactivity** — one per reader component: mount it with a known initial state, mutate the shared state from the plain test context, `flushSync()`, assert the DOM changed (e.g. "re-renders the owner badge when the current user changes" in `Part/GearCard.test.ts`).
- **Verification bar** — full `npm run test`, `npm run check:ci`, `npm run build` all green; the change reverts cleanly as one commit.
