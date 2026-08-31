# AGENTS.md

This file provides guidance to agents when working with code in this repository.

## Commands (run from workspace directory)

- `npm run dev` - Start dev server (Vite with HMR)
- `npm run build` - Production build
- `npm run check` - Run paraglide compile + svelte-check for type checking
- `npm run format` - Format all files with Prettier
- `npm run fmtcheck` - Check formatting (used in CI)
- `npm run preview` - Preview production build

**No test framework is configured** - the CI workflow runs `npm run check` (type checking) and Prettier format check only.

## Architecture

- **Framework**: Svelte 5 + TypeScript + Vite
- **UI**: Tailwind CSS v4 + Flowbite + flowbite-svelte components
- **Routing**: `svelte-spa-router` with route definitions in [`App.svelte`](src/App.svelte:37)
- **i18n**: `@inlang/paraglide-js` - compiled messages in `paraglide/`, source in `messages/{locale}.json`
- **State**: Custom `mapable()` pattern in [`lib/mapable.ts`](src/lib/mapable.ts:20) wrapping Svelte writable stores; entities expose `setMap()`/`updateMap()`/`deleteItem()` methods
- **API**: Centralized [`myfetch()`](src/lib/store.ts:43) wrapper in `lib/store.ts` with `checkStatus()` error handling (401 redirects, message display)
- **Entity classes**: Activity, Part, Service, Attachment, Usage, Shop, etc. - defined in `lib/*.ts` with async methods that call `myfetch()` and update stores via `updateSummary()` in [`lib/user.ts`](src/lib/user.ts)

## Code Style

- **Svelte 5 migration in progress**: New components should use runes mode (`$state`, `$derived`, `$effect`). Existing components may use legacy `export let` with `$:` reactive statements - do not rewrite unless touching the file
- Use `<script lang="ts">` blocks; new components: runes mode (`$state` for reactive vars, `$derived` for computed, `$effect` for side effects)
- Legacy components: `export let` for props, `$:` for derived values
- Components with two `<script>` blocks: `module` block for top-level awaits (e.g., `await getTypes()`), regular block for component logic
- TypeScript strict: `noUnusedLocals`, `noUnusedParameters`, `checkJs` enabled
- Prettier configured via `.prettierrc` with `prettier-plugin-svelte`; run `npm run format` before committing
- Entity classes use `constructor(data: any)` pattern mapping API response fields; Date fields wrapped in `new Date()`
- License header block required at top of new `.svelte` files (copy from [`App.svelte`](src/App.svelte:1))

## Key Gotchas

- API proxy configured for `^/(api)|(strava)` paths → targets `http://localhost:8000` in [`vite.config.ts`](vite.config.ts:23)
- `myfetch()` returns `null` for HTTP 204 (NO_CONTENT); callers must handle this
- Store updates must use `updateMap()`/`setMap()`/`deleteItem()` from the entity's exported store variable (e.g., `parts.updateMap([data])`)
- Paraglide messages imported via `import { m } from "../../paraglide/messages"` - translation keys follow pattern `m.action_name()`
- `svelte-spa-router` params typed as `{ id: number }` style `export let params`

You are able to use the Svelte MCP server, where you have access to comprehensive Svelte 5 and SvelteKit documentation. Here's how to use the available tools effectively:

## Available Svelte MCP Tools:

### 1. list-sections

Use this FIRST to discover all available documentation sections. Returns a structured list with titles, use_cases, and paths.
When asked about Svelte or SvelteKit topics, ALWAYS use this tool at the start of the chat to find relevant sections.

### 2. get-documentation

Retrieves full documentation content for specific sections. Accepts single or multiple sections.
After calling the list-sections tool, you MUST analyze the returned documentation sections (especially the use_cases field) and then use the get-documentation tool to fetch ALL documentation sections that are relevant for the user's task.

### 3. svelte-autofixer

Analyzes Svelte code and returns issues and suggestions.
You MUST use this tool whenever writing Svelte code before sending it to the user. Keep calling it until no issues or suggestions are returned.

### 4. playground-link

Generates a Svelte Playground link with the provided code.
After completing the code, ask the user if they want a playground link. Only call this tool after user confirmation and NEVER if code was written to files in their project.
