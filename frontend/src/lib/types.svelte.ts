import type { Type } from "./types";

/**
 * The active gear category, as a Svelte 5 state object (the
 * store-to-state migration of the former svelte/store writable in
 * types.ts). The value is nullable (unset until getTypes() resolves) and
 * replaced wholesale, which a `.svelte.ts` module may not export directly
 * (Svelte's `state_invalid_export` rule), so the state lives in the module
 * and is read via `getCategory` and replaced via `setCategory`. Reads
 * register dependencies at the enclosing reactive call site; readers use
 * `getCategory()` where they formerly used `$category` / `get(category)`.
 */
let category = $state<Type | undefined>(undefined);

export function getCategory(): Type | undefined {
  return category;
}

/** Set the active category to `value`; `undefined` clears it. */
export function setCategory(value: Type | undefined) {
  category = value;
}
