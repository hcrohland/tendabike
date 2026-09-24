/**
 * The global app message, as a Svelte 5 state object (the store-to-state
 * migration of the former svelte/store writable in store.ts). Writes are
 * plain property mutation; readers read the properties directly (no `$`
 * rune, no `get()`).
 */
export const message = $state({
  active: false,
  message: "No message",
  status: "",
});
