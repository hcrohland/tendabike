// Stub of $app/state for the component-verification harness (frontend/verify/).
// The real module is provided by SvelteKit; the harness is standalone Vite.
// `page` carries just enough shape for a component to read its url/params.
// Extend it if the component under test reads more.
export const page = {
  url: new URL("http://127.0.0.1:5199/"),
  params: {},
  status: 200,
  error: null,
  data: {},
};
