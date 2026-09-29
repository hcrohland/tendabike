/**
 * The global app message, as a Svelte 5 state object. Writes are plain
 * property mutation; readers read the properties directly.
 */
export const message = $state({
  active: false,
  message: "No message",
  status: "",
});
