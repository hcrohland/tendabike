# Paraglide JS under SvelteKit (SPA mode, `ssr: false`)

**Origin.** Research ticket #726 — does `@inlang/paraglide-js` carry over cleanly from a
plain Vite Svelte app to a SvelteKit app (SPA mode, `ssr: false`, `@sveltejs/adapter-static`),
and how does its config move? Findings verified against primary sources (the official
`@inlang/paraglide-js` docs and SvelteKit example, SvelteKit docs) as of 2026-11-04.

**Bottom line.** **Clean carry-over.** The `paraglideVitePlugin` moves into the SvelteKit
`vite.config.ts` alongside `sveltekit()` with no API change. The SSR middleware and `reroute`
hook from the official SvelteKit example are **not needed** in `ssr: false` SPA mode — the
docs explicitly say to skip them for client-only SPAs. The `outdir` and import paths can stay
as-is (relative imports from `./paraglide`), or be upgraded to the SvelteKit-idiomatic
`$lib/paraglide` alias. No `svelte.config.js` changes are required for paraglide.

---

## 1. Does `paraglideVitePlugin` work inside a SvelteKit `vite.config.ts`?

**Yes.** The official paraglide-js SvelteKit example adds `paraglideVitePlugin` directly to
the `plugins` array alongside `sveltekit()`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';

export default defineConfig({
	plugins: [
		sveltekit(),
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide',
			emitTsDeclarations: true,
			strategy: ['url', 'cookie', 'baseLocale'],
		})
	]
});
```

> Source: <https://github.com/opral/paraglide-js/blob/main/examples/sveltekit/README.md>
> (§ "Add the `paraglideVitePlugin()` to `vite.config.js`")

**Plugin ordering.** The official example places `paraglideVitePlugin` *after* `sveltekit()`.
The sibling research doc (§5) recommends `sveltekit()` be **last** in the array, with
ecosystem plugins before it. Both orderings work in practice; the paraglide plugin is an
independent Vite plugin that hooks into `resolveId`/`load` for its virtual modules and
`config` for its compiler — it has no ordering dependency on `sveltekit()`. The safe
arrangement (consistent with the sibling doc) is:

```ts
plugins: [
  paraglideVitePlugin({ … }),
  tailwindcss(),
  sveltekit({ adapter: … }),  // last
]
```

**No conflicts.** Paraglide's Vite plugin operates on its own virtual module namespace
(`virtual:paraglide/<output-id>/…`) and does not intercept Svelte file compilation,
SvelteKit's routing, or Tailwind processing. The two plugins are orthogonal.

> Source: <https://github.com/opral/paraglide-js/blob/main/docs/virtual-messages.md>
> (§ "Generated output" — virtual module namespace)

---

## 2. Is there a SvelteKit-specific paraglide integration?

**Yes — but the SSR parts are not needed for `ssr: false`.**

Paraglide JS is "SvelteKit's [official i18n integration](https://svelte.dev/docs/cli/paraglide)"
and ships a dedicated SvelteKit example
(<https://github.com/opral/paraglide-js/tree/main/examples/sveltekit>). The example includes
three SvelteKit-specific pieces:

| Piece | Purpose | Needed for `ssr: false`? |
| --- | --- | --- |
| `paraglideMiddleware()` in `src/hooks.server.ts` | Locale detection + HTML `lang`/`dir` transform on the server | **No** |
| `reroute` hook in `src/hooks.ts` | URL de-localization (strips locale prefix from incoming requests) | **No** |
| `paraglideVitePlugin` in `vite.config.ts` | Compilation + virtual module serving | **Yes** |

The middleware docs are explicit:

> "The middleware is only needed for server-side rendering (SSR). If you're building a
> client-only SPA, skip this guide and use the runtime directly."

> Source: <https://github.com/opral/paraglide-js/blob/main/docs/middleware.md> (opening note)

**For our SPA (`ssr: false`):** only the Vite plugin is needed. The runtime
(`getLocale()`, `setLocale()`, `m.*` messages) works entirely client-side. No `hooks.server.ts`,
no `hooks.ts` reroute, no `app.html` `%lang%`/`%dir%` placeholders.

**What the SvelteKit example *does* show that is relevant to us:**

- The `outdir` convention: `./src/lib/paraglide` (inside `src/lib/`, importable via `$lib`)
- `emitTsDeclarations: true` — "keeps the Svelte and TypeScript language servers in sync when
  you add or rename message keys while the dev server is running"
- Import style: `import { m } from '$lib/paraglide/messages.js'`

> Source: <https://github.com/opral/paraglide-js/blob/main/examples/sveltekit/README.md>
> (§ Usage, § "Add the `paraglideVitePlugin()` to `vite.config.js`")

---

## 3. Does the generated `paraglide/` outdir and import path change?

**The `outdir` can stay as-is, but the SvelteKit-idiomatic location is `./src/lib/paraglide`.**

### Current setup (plain Vite)

- `outdir: "./paraglide"` → output at `frontend/paraglide/`
- Imports: relative paths like `import * as m from "../paraglide/messages"` (depth varies by
  component location)

### Under SvelteKit — two options

**Option A: Keep `outdir: "./paraglide"` (minimal change)**

The `outdir` is relative to the project root (where `vite.config.ts` lives, i.e. `frontend/`).
The generated files land in `frontend/paraglide/` exactly as today. All existing relative
imports (`../paraglide/messages`, `../../paraglide/messages`, etc.) continue to work
unchanged. **Zero import-path edits needed.**

**Option B: Move to `outdir: "./src/lib/paraglide"` (SvelteKit-idiomatic)**

The official SvelteKit example uses this. The generated files land in
`frontend/src/lib/paraglide/`, and all imports become the uniform alias:

```ts
import { m } from '$lib/paraglide/messages.js';
import { getLocale, setLocale, locales } from '$lib/paraglide/runtime.js';
```

This eliminates the varying relative-depth imports. It requires updating every
`paraglide/messages` and `paraglide/runtime` import across ~20+ files.

**Both options work.** The virtual-modules output structure (default in Vite) means the
physical `messages.js` file in the outdir internally imports from a Vite virtual module
(`virtual:paraglide/<output-id>/messages/_index.js`); the user's import path is just a
regular file path to that `messages.js`. The virtual module is resolved by the paraglide
Vite plugin regardless of where the outdir is.

> Source: <https://github.com/opral/paraglide-js/blob/main/docs/virtual-messages.md>
> (§ "Generated output", § "Virtual messages in Vite")

### `emitTsDeclarations`

The SvelteKit example adds `emitTsDeclarations: true`. This generates `.d.ts` files in the
outdir so the TypeScript/Svelte language server can resolve message keys and parameters
without a separate compile step. Recommended for SvelteKit (where the dev server and language
server run concurrently). Requires TypeScript ≥ 5.6 (we have 6).

> Source: <https://github.com/opral/paraglide-js/blob/main/examples/sveltekit/README.md>
> (note under the vite config)

---

## 4. Interactions with SvelteKit specifics

### `resolve.conditions: ["browser"]`

**Carries over unchanged.** In a plain Vite app, this ensures browser-specific `exports`
conditions are used. In SvelteKit with `ssr: false`, the app is purely client-side — the
"server" pass only generates the static shell (`200.html`), which contains no app code.
The client build (where all paraglide code runs) correctly uses `browser` conditions.

The sibling research doc (§5) confirms: `resolve: { conditions: ["browser"] }` is unchanged.

No paraglide-specific interaction: the paraglide plugin's virtual modules are served directly
by Vite's module graph and do not go through `exports`-map resolution.

### SSR (`ssr: false`)

**No interaction.** With `ssr: false`:
- No server-side rendering → no `paraglideMiddleware` needed
- No `AsyncLocalStorage` request-scoping → no locale leakage concerns
- The paraglide runtime (`getLocale()`, `setLocale()`, `m.*`) runs entirely in the browser
- The `strategy: ["cookie", "preferredLanguage", "baseLocale"]` resolves client-side from
  `document.cookie` and `navigator.language` — both available in the browser

The cookie strategy works identically in a SPA: `setLocale()` writes the cookie, and
`getLocale()` reads it on subsequent loads. No server is involved.

### `project.inlang/` config dir and `messages/{locale}.json`

**Unchanged.** The `project` option in `paraglideVitePlugin` is a path relative to the
project root (`frontend/`):

- `project: "./project.inlang"` → `frontend/project.inlang/settings.json` (unchanged)
- `pathPattern: "./messages/{locale}.json"` in `settings.json` → `frontend/messages/{locale}.json` (unchanged)

Message-format paths resolve from the directory containing `project.inlang`:

> "Message-format paths resolve from the directory containing `project.inlang`. For
> `pathPattern: \"./messages/{locale}.json\"`, place `messages/` beside `project.inlang/`."

> Source: <https://github.com/opral/paraglide-js/blob/main/docs/per-locale-build-architecture.md>

The `machine-translate` npm script (`inlang machine translate --project project.inlang`) is
unaffected — it operates on the `project.inlang/` directory directly, independent of the
Vite/SvelteKit build.

### `paths.relative` (svelte.config.js)

The SvelteKit example recommends `paths: { relative: false }` in `svelte.config.js` when
using the static adapter's SPA fallback **with locale-prefixed URLs** (the `url` strategy):

> "If you use the static adapter's SPA fallback mode, make asset paths absolute to avoid
> locale-prefixed 404s"

> Source: <https://github.com/opral/paraglide-js/blob/main/examples/sveltekit/README.md>
> (§ "Static site generation (SSG)")

**Not applicable to us.** Our strategy is `["cookie", "preferredLanguage", "baseLocale"]` —
no `url` strategy, no locale-prefixed paths. Asset paths are not locale-prefixed, so
`paths.relative` has no effect on paraglide. The default (`true`) is fine.

### SvelteKit module resolution (`$lib` alias)

SvelteKit automatically aliases `$lib` → `src/lib/`. If we choose Option B
(`outdir: "./src/lib/paraglide"`), imports use `$lib/paraglide/messages.js`. If we choose
Option A (`outdir: "./paraglide"`), relative imports work as today. No `tsconfig.json`
changes are needed for either option (SvelteKit's generated `tsconfig.json` already includes
the `$lib` alias).

---

## 5. Recommended exact config for tendabike (SvelteKit SPA, `ssr: false`)

### `vite.config.ts`

```ts
import adapter from "@sveltejs/adapter-static";
import { paraglideVitePlugin } from "@inlang/paraglide-js";
import tailwindcss from "@tailwindcss/vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./paraglide",
      emitTsDeclarations: true,
      strategy: ["cookie", "preferredLanguage", "baseLocale"],
    }),
    tailwindcss(),
    sveltekit({
      adapter: adapter({
        pages: "dist",
        assets: "dist",
        fallback: "200.html",
      }),
    }),
  ],

  resolve: {
    conditions: ["browser"],
  },

  test: {
    // unchanged vitest config
  },

  build: {
    // unchanged
  },

  server: {
    host: "127.0.0.1",
    proxy: {
      "^/(api)|(strava)": {
        target: "http://localhost:8000",
        changeOrigin: true,
      },
    },
  },
});
```

**Changes from the current config:**
- `svelte()` → `sveltekit({ adapter: … })` (per the sibling doc)
- Added `emitTsDeclarations: true` to `paraglideVitePlugin` (new in the SvelteKit context)
- Everything else in the paraglide config is unchanged

### `svelte.config.js` — unchanged

```js
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
```

No paraglide-specific changes. (In a SvelteKit 3 migration, this file would be eliminated
and `preprocess` would move into the `sveltekit()` plugin options, but that is out of scope
for the paraglide question.)

### `src/app.d.ts` — SvelteKit-generated

SvelteKit generates `src/app.d.ts` with the `App` namespace types. No paraglide-specific
additions needed. The paraglide virtual modules provide their own type declarations via
`emitTsDeclarations: true` (the `messages.d.ts` + `messages.virtual.d.ts` in the outdir).

### Import paths — no change (Option A) or uniform alias (Option B)

**Option A (recommended for minimal diff):** Keep all existing relative imports.
`import * as m from "../paraglide/messages"` continues to work because the outdir is still
`frontend/paraglide/`.

**Option B (SvelteKit-idiomatic, more edits):** Move outdir to `./src/lib/paraglide` and
replace all imports with:
```ts
import { m } from '$lib/paraglide/messages.js';
import { getLocale, setLocale, locales } from '$lib/paraglide/runtime.js';
```

### What is NOT needed (SSR-only pieces from the SvelteKit example)

- ❌ `src/hooks.server.ts` with `paraglideMiddleware`
- ❌ `src/hooks.ts` with `reroute` / `deLocalizeUrl`
- ❌ `%lang%` / `%dir%` placeholders in `src/app.html`
- ❌ `paths: { relative: false }` in `svelte.config.js`

All four are for SSR / locale-prefixed-URL scenarios that do not apply to a `ssr: false`
SPA with a cookie-based strategy.

---

## Citations

| # | Source | URL |
| --- | --- | --- |
| 1 | Paraglide JS SvelteKit example README | <https://github.com/opral/paraglide-js/blob/main/examples/sveltekit/README.md> |
| 2 | Paraglide JS middleware docs | <https://github.com/opral/paraglide-js/blob/main/docs/middleware.md> |
| 3 | Paraglide JS virtual messages docs | <https://github.com/opral/paraglide-js/blob/main/docs/virtual-messages.md> |
| 4 | Paraglide JS strategy docs | <https://github.com/opral/paraglide-js/blob/main/docs/strategy.md> |
| 5 | Paraglide JS i18n routing docs | <https://github.com/opral/paraglide-js/blob/main/docs/i18n-routing.md> |
| 6 | Paraglide JS per-locale build architecture | <https://github.com/opral/paraglide-js/blob/main/docs/per-locale-build-architecture.md> |
| 7 | Paraglide JS compiler options API | <https://github.com/opral/paraglide-js/blob/main/docs-api/compiler-options.md> |
| 8 | SvelteKit SPA docs | <https://svelte.dev/docs/kit/single-page-apps> |
| 9 | SvelteKit page options (`ssr`) | <https://svelte.dev/docs/kit/page-options> |
| 10 | SvelteKit project structure (`vite.config.js`) | <https://svelte.dev/docs/kit/project-structure> |
| 11 | SvelteKit adapter-static docs | <https://svelte.dev/docs/kit/adapter-static> |
| 12 | SvelteKit CLI paraglide integration | <https://svelte.dev/docs/cli/paraglide> |
