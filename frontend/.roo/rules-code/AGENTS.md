# Project Coding Rules (Non-Obvious Only)

- **Svelte 5 runes mode preferred for NEW components**: use `$state` for reactive vars, `$derived` for computed values, `$effect` for side effects
- Legacy components use `export let` for props and `$:` for reactive statements - preserve these when not modifying the file
- All entity stores follow `mapable()` pattern from `lib/mapable.ts` - update with `entityStore.updateMap([data])` after API calls
- API calls go through `myfetch()` in `lib/store.ts` - NEVER use raw `fetch()` directly
- Error handling: use `.then(updateSummary).catch(handleError)` pattern for all API mutations
- Import paraglide messages as `import { m } from "../../paraglide/messages"` (relative path from component to frontend root)
- Entity constructors accept `data: any` and map fields - maintain this pattern for new entities
- Two `<script lang="ts">` blocks: use `module` block only for top-level await initialization (like `await getTypes()`)
- New components go in feature directories under `src/` (e.g., `src/Part/`, `src/Widgets/`)
- Custom types defined in `lib/types.ts` use `localizedName()` pattern for i18n fallback to raw name
