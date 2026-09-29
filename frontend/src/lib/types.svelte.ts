import type { Type } from "./types";

/**
 * The active gear category, as a Svelte 5 state object. The value is
 * nullable (unset until getTypes() resolves) and replaced wholesale, which
 * a `.svelte.ts` module may not export directly (Svelte's
 * `state_invalid_export` rule), so the state lives in the module and is
 * read via `getCategory` and replaced via `setCategory`. Reads register
 * dependencies at the enclosing reactive call site.
 */
let category = $state<Type | undefined>(undefined);

export function getCategory(): Type | undefined {
  return category;
}

/** Set the active category to `value`; `undefined` clears it. */
export function setCategory(value: Type | undefined) {
  category = value;
}
