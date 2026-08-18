# Project Architecture Rules (Non-Obvious Only)

- All entities use centralized `mapable()` store pattern - no separate store files; stores defined inline in entity files
- Data refresh flows through `user.ts`: `initData()` → `refresh()` → `setSummary()` → all entity stores updated
- `updateSummary()` in `user.ts` updates stores incrementally after mutations; callers should pass new data to avoid full refresh
- API routes use URL pattern matching in `vite.config.ts` proxy (`^/(api)|(strava)`) - frontend never makes cross-origin requests
- Feature directories (`src/Part/`, `src/Activity/`, etc.) contain both view components and related widgets in `src/Widgets/`
- License header (AGPLv3) required on all new `.svelte` files - copy block from `src/App.svelte`
