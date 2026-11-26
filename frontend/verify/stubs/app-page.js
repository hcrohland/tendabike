// Stub of $app/page (legacy stores) for the component-verification harness
// (frontend/verify/). Legacy auto-subscription ($page) needs subscribable
// values, hence readable().
import { readable } from "svelte/store";

export const page = readable({
  url: new URL("http://127.0.0.1:5199/"),
  params: {},
  status: 200,
  error: null,
  data: {},
});

export const navigating = readable(null);

export const updated = readable(false);
