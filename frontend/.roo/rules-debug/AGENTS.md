# Project Debug Rules (Non-Obvious Only)

- Dev server HMR configured with `protocol: "ws"` and `host: "localhost"` in `vite.config.ts` - if HMR fails, check host configuration
- API requests proxy to `http://localhost:8000` - ensure backend is running on port 8000 for full integration
- `svelte-check` must be run from `frontend/` directory with `--tsconfig ./tsconfig.json` flag
- Paraglide compile (`npx paraglide-js compile --outdir ./paraglide`) must succeed before type-checking; missing `paraglide/messages` import indicates paraglide not compiled
- Global message display component (`Message.svelte`) shows API errors - check this UI element for 401/4xx error details
- HTTP 204 responses return `null` from `myfetch()` - if data is unexpectedly `null`, check if backend returned 204 NO_CONTENT
