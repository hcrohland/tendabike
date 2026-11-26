# Component-level verification

Recipe for verifying what a component renders — color, layout, computed
styles — without the full local stack: no database, no backend, no SvelteKit
layout, no auth. The harness is a committed standalone Vite app
(`verify/`) that mounts the app's real components with the app's real theme
(`src/app.css`) on a fixed port.

For what the harness does not cover — interaction (menu/popper open-close),
or a component that needs data from the app — use the full stack in
[`dev-stack.md`](dev-stack.md).

## Steps

1. **Drop in the component under test.** Edit `verify/Example.svelte` to
   mount the component(s) you are verifying, with a `data-testid` anchor on
   whatever you will assert. Import real app components across the boundary
   (`../src/…`), as the example does. `$app/*` imports resolve to the stubs
   in `verify/stubs/`; paraglide messages resolve to the compiled
   `paraglide/` (the script compiles it first). Extend a stub if the
   component reads more than it carries.

2. **Run the harness.** From the repo root: `npm run verify`. It serves
   `verify/index.html` on `:5199`. Done when: `curl -s localhost:5199`
   returns 200.

3. **Look.** Drive it with Playwright (installed globally — symlink it next
   to the script, as in `dev-stack.md` step 5): assert computed styles
   (`getComputedStyle`) on the `data-testid` anchors. For dark mode, add
   `dark` to `document.documentElement` first. A green jsdom suite does not
   cover rendered output — this is the render.

4. **Teardown.** Kill the pid listening on `:5199` (`ss -ltnp | grep 5199`).
   Done when: the port refuses connections.

## Keeping the harness honest

`src/test/verify-harness.test.ts` mounts the example in jsdom and checks the
tone classes, so a broken import, stub, or entry is caught by the test gate
when the stack moves. It is not a rendering check: the theme is verified in
a real browser, in step 3.
