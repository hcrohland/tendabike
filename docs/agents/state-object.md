# State Objects

How the Tendabike client holds shared state. Svelte 5 state objects in `.svelte.ts` modules replace the `svelte/store` writables, one store per ticket (the wide migration, #349). Read this when converting a store, or when touching a `.svelte.ts` state module or its readers.

## The three shapes

Pick by what the value is:

1. **Collection keyed by id** — the `mapableState` factory (`frontend/src/lib/mapable.svelte.ts`): a `$state` record of entities by id with `setMap`/`updateMap`/`deleteItem` attached. Reference: `part.ts`.
2. **Object, always defined** — `export const x = $state({...})` in `x.svelte.ts`; writes are property mutation. Reference: `message.svelte.ts` (its old home, `store.ts`, re-exports the state).
3. **Object, nullable or replaced wholesale** — a module-private `let x = $state(...)` in `x.svelte.ts`, exposed through `getX()`/`setX()`; the old home file re-exports the accessors. Reference: `user.svelte.ts` (its old home, `user.ts`, re-exports the accessors).

**Why shape 3 is accessors, not an exported state.** Svelte refuses to compile a `.svelte.ts` module that exports state it reassigns (`state_invalid_export`), and it equally forbids exporting a `$derived` (`derived_invalid_export`). A nullable value must be reassigned to cross `undefined` — a property mutation cannot conjure an object out of `undefined` — and a stable holder object is always truthy, which breaks `{#if x}` gates. The value therefore stays module-private, and the sanctioned fix (Svelte's own error message) is a function returning it.

## Mechanics

- **The `.svelte.ts` suffix is the contract**: only `.svelte.ts`/`.svelte.js` files are compiled by Svelte, so `$state` only works there. Import without the `.ts` stub: `import { ... } from "./user.svelte"` resolves to `user.svelte.ts`.
- **The old home file re-exports** (`user.ts`: `export { getUser, setUser };`) so existing importers keep their specifier — the reader diff stays minimal.
- **Reader sites** — the `$user` rune is gone; read the value where it is exposed, keeping each site's null style (`?.` / `!` / gate) on every property read (no narrowing across calls in the accessor shape):
  - state object: `$x` → `x`.
  - accessor shape: `$user` → `getUser()`, `$user?.id` → `getUser()?.id`, `$user!.id` → `getUser()!.id`.
  - writes: `$user = v` / `user.set(v)` / `user.update(f)` → `setUser(v)`; property mutation for shape 2.
- **Reactivity** — a state read inside a plain function registers dependencies at the enclosing reactive call site (the "door" shape, verified in #322/#346). Shape 2 tracks property mutation at any depth through its stable proxy. Shape 3 tracks the _replacement_ only (the replaced value is a plain object) — exactly what wholesale-replacement writes need.
- **Enumerate collections with `stateValues(c)`.** The write operations attach to the record, so `Object.keys`/`Object.values`/`filterValues` see them; value-iteration over a state collection surfaces the operations as entries.

## Tests

- **Read idiom** — `get(x)` no longer applies to a state object; assert on the direct read: `x["id"]`, `x.prop`, or `getX()`.
- **Component reactivity** — one per ticket: mount the reader component with a known initial state, mutate the shared state from the plain test context, `flushSync()`, assert the DOM changed (the #346 A1 pattern; e.g. "re-renders the owner badge when the current user changes" in `Part/GearCard.test.ts`).
- **Verification bar per ticket** — full `npm run test`, `npm run check:ci`, `npm run build` all green; one store per ticket; the change reverts cleanly as one commit.
