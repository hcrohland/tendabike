# Project Documentation Rules (Non-Obvious Only)

- `src/lib/` contains business logic classes (NOT utilities) - each entity has its own file (activity.ts, part.ts, service.ts, etc.)
- `src/lib/store.ts` is the API client layer, not a Svelte store (despite the name) - actual stores use `mapable()` pattern
- `src/lib/mapable.ts` contains the core store abstraction - understanding this is key to modifying any entity data flow
- Routes defined as object in `App.svelte` - async components use `wrap()` from `svelte-spa-router/wrap`
- Two separate i18n systems: backend has no i18n; frontend uses paraglide with locale files in `messages/{locale}.json`
- `paraglide/messages` is auto-generated - do NOT edit files in `paraglide/` directory directly
